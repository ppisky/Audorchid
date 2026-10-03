use anyhow::{Result, ensure};
use openh264::formats::YUVBuffer;

/// Display-referred RGBA -> BT.709 matrix / limited I420, with 2x2 chroma averaging.
/// Preserve the already-rendered R'G'B' values; applying a second camera OETF
/// here darkens UI midtones and makes decoded video disagree with PNG previews.
pub struct ColorConverter {
    transfer: [f32; 256],
}
impl Default for ColorConverter {
    fn default() -> Self {
        let transfer = std::array::from_fn(|i| i as f32 / 255.);
        Self { transfer }
    }
}
impl ColorConverter {
    pub fn convert(&self, rgba: &[u8], w: usize, h: usize) -> Result<YUVBuffer> {
        ensure!(
            w > 0 && h > 0 && w.is_multiple_of(2) && h.is_multiple_of(2) && rgba.len() == w * h * 4,
            "I420 requires tightly packed RGBA with positive even dimensions"
        );
        let mut yuv = vec![0; w * h * 3 / 2];
        let ubase = w * h;
        let vbase = ubase + w * h / 4;
        for by in (0..h).step_by(2) {
            for bx in (0..w).step_by(2) {
                let mut u = 0.;
                let mut v = 0.;
                for dy in 0..2 {
                    for dx in 0..2 {
                        let pixel = (by + dy) * w + bx + dx;
                        let off = pixel * 4;
                        let r = self.transfer[rgba[off] as usize];
                        let g = self.transfer[rgba[off + 1] as usize];
                        let b = self.transfer[rgba[off + 2] as usize];
                        let luma = 0.2126 * r + 0.7152 * g + 0.0722 * b;
                        yuv[pixel] = (16. + 219. * luma).round().clamp(16., 235.) as u8;
                        u += (b - luma) / 1.8556;
                        v += (r - luma) / 1.5748;
                    }
                }
                let c = (by / 2) * (w / 2) + bx / 2;
                yuv[ubase + c] = (128. + 224. * u / 4.).round().clamp(16., 240.) as u8;
                yuv[vbase + c] = (128. + 224. * v / 4.).round().clamp(16., 240.) as u8;
            }
        }
        Ok(YUVBuffer::from_vec(yuv, w, h))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use openh264::formats::YUVSource;
    #[test]
    fn black_white_and_red_have_709_levels() {
        let c = ColorConverter::default();
        for (rgb, expected) in [
            ([0, 0, 0, 255], [16, 128, 128]),
            ([255, 255, 255, 255], [235, 128, 128]),
            ([128, 128, 128, 255], [126, 128, 128]),
            ([255, 0, 0, 255], [63, 102, 240]),
        ] {
            let yuv = c.convert(&rgb.repeat(4), 2, 2).unwrap();
            assert_eq!([yuv.y()[0], yuv.u()[0], yuv.v()[0]], expected);
        }
    }
    #[test]
    fn all_gray_midtone_values_round_trip_without_extra_transfer() {
        let c = ColorConverter::default();
        for gray in 0..=255u8 {
            let yuv = c.convert(&[gray, gray, gray, 255].repeat(4), 2, 2).unwrap();
            assert_eq!((yuv.u()[0], yuv.v()[0]), (128, 128));
            let decoded = ((yuv.y()[0] as f32 - 16.) * 255. / 219.).round() as u8;
            assert!(decoded.abs_diff(gray) <= 1);
        }
    }
}
