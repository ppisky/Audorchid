use anyhow::{Context, Result, ensure};
use audorchid_audio::PreparedAudio;
use audorchid_core::Timeline;
use std::{
    io::{BufWriter, Write},
    ops::Range,
    path::Path,
};

pub fn transition_range(
    timeline: &Timeline,
    transition: usize,
    before: f64,
    after: f64,
) -> Result<Range<usize>> {
    ensure!(
        transition > 0 && transition < timeline.tracks.len(),
        "--transition must be within 1..{}",
        timeline.tracks.len().saturating_sub(1)
    );
    ensure!(
        before.is_finite() && before >= 0. && after.is_finite() && after >= 0.,
        "--before and --after must be nonnegative and finite"
    );
    let next = &timeline.tracks[transition];
    let sr = timeline.sample_rate as f64;
    let start = next.start.saturating_sub((before * sr).round() as u64);
    let end = next
        .start
        .saturating_add(timeline.incoming_overlap(transition))
        .saturating_add((after * sr).round() as u64)
        .min(timeline.total_samples);
    ensure!(end > start, "transition preview is shorter than one sample");
    Ok(usize::try_from(start)?..usize::try_from(end)?)
}

pub fn save(
    audio: &PreparedAudio,
    seconds: f64,
    duration: f64,
    path: &Path,
    overwrite: bool,
) -> Result<()> {
    ensure!(
        seconds.is_finite() && seconds >= 0. && seconds < audio.timeline.seconds(),
        "--seconds must be within the timeline"
    );
    ensure!(
        duration.is_finite() && duration > 0.,
        "--duration must be positive and finite"
    );
    let sr = audio.timeline.sample_rate;
    let start = (seconds * sr as f64).round() as usize;
    let wanted = (duration * sr as f64).round() as usize;
    let end = start.saturating_add(wanted).min(audio.pcm.len() / 2);
    save_range(audio, start..end, path, overwrite)
}

pub fn save_range(
    audio: &PreparedAudio,
    range: Range<usize>,
    path: &Path,
    overwrite: bool,
) -> Result<()> {
    let Range { start, end } = range;
    let sr = audio.timeline.sample_rate;
    ensure!(
        end <= audio.pcm.len() / 2,
        "audio preview exceeds the timeline"
    );
    ensure!(end > start, "audio preview is shorter than one sample");
    let bytes = u32::try_from((end - start).checked_mul(4).context("WAV size overflow")?)?;
    ensure!(bytes <= u32::MAX - 36, "WAV exceeds its 32-bit size limit");
    let parent = path.parent().context("WAV output has no parent")?;
    std::fs::create_dir_all(parent)?;
    let mut temp = tempfile::Builder::new()
        .prefix(".audorchid-audio-")
        .suffix(".wav.part")
        .tempfile_in(parent)?;
    temp.write_all(b"RIFF")?;
    temp.write_all(&(36 + bytes).to_le_bytes())?;
    temp.write_all(b"WAVEfmt ")?;
    temp.write_all(&16u32.to_le_bytes())?;
    temp.write_all(&1u16.to_le_bytes())?;
    temp.write_all(&2u16.to_le_bytes())?;
    temp.write_all(&sr.to_le_bytes())?;
    temp.write_all(&(sr * 4).to_le_bytes())?;
    temp.write_all(&4u16.to_le_bytes())?;
    temp.write_all(&16u16.to_le_bytes())?;
    temp.write_all(b"data")?;
    temp.write_all(&bytes.to_le_bytes())?;
    // Bound temporary memory even when previewing an entire long playlist.
    {
        let mut writer = BufWriter::new(temp.as_file_mut());
        for sample in &audio.pcm[start * 2..end * 2] {
            writer.write_all(&((sample.clamp(-1., 1.) * 32767.).round() as i16).to_le_bytes())?;
        }
        writer.flush()?;
    }
    temp.as_file().sync_all()?;
    if overwrite {
        temp.persist(path)?;
    } else {
        temp.persist_noclobber(path)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn transition_preview_uses_actual_variable_overlaps_and_clamps_context() {
        let t = Timeline::with_overlaps(&[1000, 1000, 1000], 100, &[400, 200]).unwrap();
        assert_eq!(transition_range(&t, 1, 2., 3.).unwrap(), 400..1300);
        assert_eq!(transition_range(&t, 2, 0., 0.).unwrap(), 1400..1600);
        assert_eq!(transition_range(&t, 1, 100., 100.).unwrap(), 0..2400);
        assert!(transition_range(&t, 0, 1., 1.).is_err());
        assert!(transition_range(&t, 3, 1., 1.).is_err());
        assert!(transition_range(&t, 1, f64::NAN, 1.).is_err());
        assert!(transition_range(&t, 1, -1., 1.).is_err());
        let straight = Timeline::new(&[1000, 1000], 100, 0).unwrap();
        assert_eq!(transition_range(&straight, 1, 2., 3.).unwrap(), 800..1300);
        assert!(transition_range(&straight, 1, 0., 0.).is_err());
    }
}
