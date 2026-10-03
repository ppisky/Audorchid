use audorchid_core::SpectrumStyle;
use rustfft::{FftPlanner, num_complex::Complex};
use std::sync::Arc;

#[derive(Clone, Copy, Default)]
pub struct Peak {
    pub left: f32,
    pub right: f32,
    pub rms: f32,
}
pub struct Analysis {
    pub peaks: Vec<Peak>,
    pub stride: usize,
    sample_rate: u32,
}
impl Analysis {
    pub fn new(pcm: &[f32], sample_rate: u32) -> Self {
        let stride = (sample_rate as usize / 100).max(1);
        let peaks = pcm
            .chunks(stride * 2)
            .map(|chunk| {
                let mut p = Peak::default();
                for c in chunk.chunks_exact(2) {
                    p.left = p.left.max(c[0].abs());
                    p.right = p.right.max(c[1].abs());
                    p.rms += c[0] * c[0] + c[1] * c[1];
                }
                p.rms = (p.rms / chunk.len() as f32).sqrt();
                p
            })
            .collect();
        Self {
            peaks,
            stride,
            sample_rate,
        }
    }
    /// Peak-preserving downsampling of the 12-second window around the cursor.
    pub fn waveform(&self, sample: u64, bars: usize) -> Vec<Peak> {
        let window = self.sample_rate as i64 * 12;
        let start = sample as i64 - window / 2;
        (0..bars)
            .map(|i| {
                let a = (start + i as i64 * window / bars as i64).div_euclid(self.stride as i64);
                let b = (start + (i + 1) as i64 * window / bars as i64)
                    .div_euclid(self.stride as i64)
                    + 1;
                let mut peak = Peak::default();
                for j in a.max(0)..b.min(self.peaks.len() as i64) {
                    let p = self.peaks[j as usize];
                    peak.left = peak.left.max(p.left);
                    peak.right = peak.right.max(p.right);
                    peak.rms = peak.rms.max(p.rms);
                }
                peak
            })
            .collect()
    }
    pub fn energy(&self, sample: u64) -> f32 {
        // Symmetric 100 ms smoothing is evaluated from source time, not frame count.
        let centre = sample as i64 / self.stride as i64;
        let mut sum = 0.;
        let mut count = 0.;
        for i in (centre - 5).max(0)..(centre + 6).min(self.peaks.len() as i64) {
            sum += self.peaks[i as usize].rms;
            count += 1.;
        }
        if count > 0. {
            (sum / count * 3.).clamp(0., 1.)
        } else {
            0.
        }
    }
}
struct FftSpectrum {
    fft: Arc<dyn rustfft::Fft<f32>>,
    buffer: Vec<Complex<f32>>,
}
impl Default for FftSpectrum {
    fn default() -> Self {
        Self {
            fft: FftPlanner::new().plan_fft_forward(4096),
            buffer: vec![Complex::default(); 4096],
        }
    }
}
impl FftSpectrum {
    fn at(&mut self, pcm: &[f32], sample: u64, sr: u32, style: &SpectrumStyle) -> Vec<f32> {
        let n = self.buffer.len();
        let mut magnitudes = vec![0f32; n / 2];
        // Analyse each channel separately so opposite-phase stereo is visible.
        for channel in 0..2 {
            for (i, v) in self.buffer.iter_mut().enumerate() {
                let idx = sample as i64 + i as i64 + 1 - n as i64;
                let mono = if idx >= 0 && (idx as usize) * 2 + 1 < pcm.len() {
                    pcm[idx as usize * 2 + channel]
                } else {
                    0.
                };
                let hann = 0.5 - 0.5 * (std::f32::consts::TAU * i as f32 / (n - 1) as f32).cos();
                *v = Complex::new(mono * hann, 0.);
            }
            self.fft.process(&mut self.buffer);
            for (m, v) in magnitudes.iter_mut().zip(&self.buffer) {
                *m = m.max(v.norm() * 4. / n as f32);
            }
        }
        let count = style.band_count();
        (0..count)
            .map(|i| {
                let hi_hz = 16000f32.min(sr as f32 * 0.49);
                let lo = 30. * (hi_hz / 30.).powf(i as f32 / count as f32);
                let hi = 30. * (hi_hz / 30.).powf((i + 1) as f32 / count as f32);
                let a = (lo * n as f32 / sr as f32).floor() as usize;
                let b = ((hi * n as f32 / sr as f32).ceil() as usize)
                    .max(a + 1)
                    .min(n / 2);
                let amplitude = magnitudes[a..b].iter().copied().fold(0f32, f32::max);
                ((20. * amplitude.max(1e-10).log10() - style.floor_db) / -style.floor_db)
                    .clamp(0., 1.)
            })
            .collect()
    }
}

/// Causal 10 ms silence test; never looks ahead into the next track.
pub fn audible(pcm: &[f32], sample: u64, sr: u32) -> bool {
    let end = sample as usize;
    if end >= pcm.len() / 2 {
        return false;
    }
    let start = end.saturating_sub(sr as usize / 100);
    pcm[start * 2..(end + 1) * 2]
        .iter()
        .any(|s| s.abs() > 0.00001)
}
/// Real signed PCM, separate L/R traces. No invented baseline during silence.
pub fn scope(pcm: &[f32], sample: u64, sr: u32, window_ms: f32, points: usize) -> Vec<[f32; 2]> {
    if !audible(pcm, sample, sr) {
        return Vec::new();
    }
    let window = (sr as f64 * window_ms as f64 / 1000.).round() as i64;
    (0..points)
        .map(|i| {
            let s = sample as i64 - window + 1 + i as i64 * (window - 1) / (points - 1) as i64;
            if s < 0 || s as usize >= pcm.len() / 2 {
                [0.; 2]
            } else {
                [pcm[s as usize * 2], pcm[s as usize * 2 + 1]]
            }
        })
        .collect()
}
#[derive(Clone, Default)]
pub struct SpectrumFrame {
    pub bars: Vec<f32>,
    pub peaks: Vec<f32>,
}
/// Fixed 100 Hz analysis clock: render FPS and seek order cannot change decay.
pub struct Spectrum {
    stride: u64,
    frames: Vec<SpectrumFrame>,
}
impl Spectrum {
    pub fn new(pcm: &[f32], sr: u32, style: &SpectrumStyle) -> Self {
        let stride = sr as u64 / 100;
        let mut fft = FftSpectrum::default();
        let count = style.band_count();
        let mut bars = vec![0f32; count];
        let mut peaks = bars.clone();
        let mut holds = vec![0f32; count];
        let mut frames = Vec::new();
        for sample in (0..pcm.len() / 2).step_by(stride as usize) {
            if !audible(pcm, sample as u64, sr) {
                bars.fill(0.);
                peaks.fill(0.);
                holds.fill(0.);
            } else {
                let raw = fft.at(pcm, sample as u64, sr, style);
                for i in 0..count {
                    bars[i] = raw[i].max(bars[i] - 10. / style.release_ms.max(1.));
                    if bars[i] >= peaks[i] {
                        peaks[i] = bars[i];
                        holds[i] = style.peak_hold_ms;
                    } else if holds[i] > 0. {
                        holds[i] -= 10.;
                    } else {
                        peaks[i] = bars[i].max(peaks[i] - 10. / style.peak_fall_ms);
                    }
                }
            }
            frames.push(SpectrumFrame {
                bars: bars.clone(),
                peaks: peaks.clone(),
            });
        }
        Self { stride, frames }
    }
    pub fn at(&self, sample: u64) -> SpectrumFrame {
        self.frames
            .get((sample / self.stride) as usize)
            .cloned()
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn silence_has_no_scope_or_spectrum_and_no_lookahead() {
        let mut pcm = vec![0.; 48000 * 2];
        for i in 24000..36000 {
            let x = (i as f32 * std::f32::consts::TAU * 1000. / 48000.).sin() * 0.5;
            pcm[i * 2] = x;
            pcm[i * 2 + 1] = -x;
        }
        let spectrum = Spectrum::new(&pcm, 48000, &SpectrumStyle::default());
        assert!(!audible(&pcm, 23000, 48000));
        assert!(scope(&pcm, 23000, 48000, 40., 256).is_empty());
        assert!(spectrum.at(23000).bars.iter().all(|v| *v == 0.));
        let peak = spectrum.at(30000);
        assert!(peak.bars.iter().copied().fold(0f32, f32::max) > 0.8);
        assert_eq!(spectrum.at(30000).bars, peak.bars);
        assert!(spectrum.at(40000).peaks.iter().all(|v| *v == 0.));
        assert!(!audible(&pcm, 48000, 48000));
    }
}
