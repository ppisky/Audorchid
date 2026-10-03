//! Offline, gated BS.1770 measurement. Normalisation uses a constant gain;
//! it does not compress dynamics, remove silence or shift the audio clock.
use anyhow::Result;
use audorchid_core::Loudness;
use ebur128::{EbuR128, Mode};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct Measurement {
    /// None for silence or fewer than 400 ms of measurable programme.
    pub integrated_lufs: Option<f64>,
    pub true_peak_dbtp: Option<f64>,
}
#[derive(Debug, Clone, Serialize)]
pub struct TrackLoudness {
    pub input: Measurement,
    pub normalization_gain_db: f64,
    pub manual_gain_db: f32,
}
#[derive(Debug, Clone, Serialize)]
pub struct LoudnessReport {
    pub tracks: Vec<TrackLoudness>,
    pub mix_before: Measurement,
    pub output: Measurement,
    pub normalization_gain_db: f64,
    pub peak_attenuation_db: f64,
    pub target_lufs: Option<f64>,
}
pub fn measure(pcm: &[f32], sr: u32) -> Result<Measurement> {
    let mut meter = EbuR128::new(2, sr, Mode::I | Mode::TRUE_PEAK)?;
    meter.add_frames_f32(pcm)?;
    let integrated = meter.loudness_global()?;
    // Flush the oversampling FIR after capturing integrated loudness, without
    // extending the programme duration or including this padding in gating.
    meter.add_frames_f32(&[0.; 128])?;
    let peak = meter.true_peak(0)?.max(meter.true_peak(1)?);
    Ok(Measurement {
        integrated_lufs: integrated.is_finite().then_some(integrated),
        true_peak_dbtp: (peak > 0.).then(|| 20. * peak.log10()),
    })
}
pub fn normalization_gain(m: &Measurement, config: &Loudness) -> f64 {
    m.integrated_lufs
        .map(|v| (config.target_lufs - v).min(config.max_gain_db))
        .unwrap_or(0.)
}
pub fn apply_gain(pcm: &mut [f32], db: f64) {
    let gain = 10f64.powf(db / 20.) as f32;
    for sample in pcm {
        *sample *= gain;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn tone(amplitude: f32) -> Vec<f32> {
        (0..48000 * 3)
            .flat_map(|i| {
                let x = amplitude * (std::f32::consts::TAU * 1000. * i as f32 / 48000.).sin();
                [x, x]
            })
            .collect()
    }
    #[test]
    fn gated_stereo_tone_and_constant_gain() {
        let mut pcm = tone(0.1);
        let before = measure(&pcm, 48000).unwrap();
        assert!((before.integrated_lufs.unwrap() + 20.).abs() < 0.2);
        let config = Loudness::default();
        apply_gain(&mut pcm, normalization_gain(&before, &config));
        let after = measure(&pcm, 48000).unwrap();
        assert!((after.integrated_lufs.unwrap() + 16.).abs() < 0.02);
        assert!((after.true_peak_dbtp.unwrap() - before.true_peak_dbtp.unwrap() - 4.).abs() < 0.2);
    }
    #[test]
    fn silence_and_short_clips_are_not_boosted() {
        for pcm in [vec![0.; 96000], tone(0.1)[..9600].to_vec()] {
            let m = measure(&pcm, 48000).unwrap();
            assert_eq!(m.integrated_lufs, None);
            assert_eq!(normalization_gain(&m, &Loudness::default()), 0.);
        }
    }
}
