//! Original procedural artwork shared by missing-cover fallback and the CLI.
use anyhow::{Result, ensure};
use image::{Rgba, RgbaImage};

pub fn generate_cover(seed: u64, size: u32) -> Result<RgbaImage> {
    ensure!(
        (64..=4096).contains(&size),
        "cover size must be between 64 and 4096 pixels"
    );
    let phase = (seed % 4096) as f32 * 1.7;
    Ok(RgbaImage::from_fn(size, size, |x, y| {
        let u = x as f32 / size as f32;
        let v = y as f32 / size as f32;
        let ring = (((u - 0.5).hypot(v - 0.5) * 30. - phase).sin() * 0.5 + 0.5).powf(10.);
        let light = (-((u - 0.3).powi(2) + (v - 0.25).powi(2)) * 5.).exp();
        Rgba([
            (25. + 100. * light + 65. * ring) as u8,
            (28. + 80. * v + 40. * ring) as u8,
            (65. + 110. * light + 35. * phase.sin()) as u8,
            255,
        ])
    }))
}
