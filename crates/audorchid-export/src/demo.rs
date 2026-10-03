use anyhow::{Result, ensure};
use image::{Rgba, RgbaImage};
use std::{io::Write, path::Path};

pub fn create(dir: &Path) -> Result<()> {
    for name in [
        "first.wav",
        "second.wav",
        "first.png",
        "second.png",
        "first.mid",
        "second.mid",
    ] {
        ensure!(
            !dir.join(name).exists(),
            "demo asset already exists: {}",
            dir.join(name).display()
        );
    }
    std::fs::create_dir_all(dir)?;
    for (i, name) in ["first", "second"].iter().enumerate() {
        let sr = if i == 0 { 44100u32 } else { 48000 };
        let frames = sr * crate::demo_music::SECONDS;
        let (music, midi) = crate::demo_music::render(i, sr);
        std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(dir.join(format!("{name}.mid")))?
            .write_all(&midi)?;
        let mut wave = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(dir.join(format!("{name}.wav")))?;
        let bytes = frames * 4;
        wave.write_all(b"RIFF")?;
        wave.write_all(&(36 + bytes).to_le_bytes())?;
        wave.write_all(b"WAVEfmt ")?;
        wave.write_all(&16u32.to_le_bytes())?;
        wave.write_all(&1u16.to_le_bytes())?;
        wave.write_all(&2u16.to_le_bytes())?;
        wave.write_all(&sr.to_le_bytes())?;
        wave.write_all(&(sr * 4).to_le_bytes())?;
        wave.write_all(&4u16.to_le_bytes())?;
        wave.write_all(&16u16.to_le_bytes())?;
        wave.write_all(b"data")?;
        wave.write_all(&bytes.to_le_bytes())?;
        let mut pcm = Vec::with_capacity(bytes as usize);
        for frame in music {
            for s in frame {
                pcm.extend_from_slice(&((s * 32767.) as i16).to_le_bytes());
            }
        }
        wave.write_all(&pcm)?;
        let image = RgbaImage::from_fn(800, 800, |x, y| {
            let u = x as f32 / 800.;
            let v = y as f32 / 800.;
            let r = (u - 0.5).hypot(v - 0.5);
            let ring = (r * 45. + i as f32 * 2.).sin().abs().powf(20.);
            if i == 0 {
                Rgba([
                    (65. + 100. * u + 35. * ring) as u8,
                    (45. + 90. * v + 35. * ring) as u8,
                    (105. + 100. * (1. - r).max(0.)) as u8,
                    255,
                ])
            } else {
                Rgba([
                    (35. + 85. * v + 45. * ring) as u8,
                    (95. + 80. * u + 45. * ring) as u8,
                    (100. + 95. * (1. - r).max(0.)) as u8,
                    255,
                ])
            }
        });
        image.save(dir.join(format!("{name}.png")))?;
    }
    println!(
        "Created two original 16-second sampled compositions (WAV + MIDI) and covers in {}",
        dir.display()
    );
    Ok(())
}
