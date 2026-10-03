//! Offline audio preparation and AAC encoding.
mod aac;
mod analysis;
mod arrangement;
mod loudness;
mod transitions;
pub use aac::*;
pub use analysis::*;
pub use arrangement::ArrangementReport;
pub use loudness::*;
pub use transitions::TransitionReport;

use anyhow::{Context, Result, ensure};
use audioadapter_buffers::owned::InterleavedOwned;
use audorchid_core::{CrossfadeCurve, LoudnessMode, Project, Timeline, TransitionMode};
use oxiaudio::AudioBuffer;
use rubato::{Fft, FixedSync, Resampler};

pub struct PreparedAudio {
    /// Stereo interleaved PCM; each timeline sample is one pair of values.
    pub pcm: Vec<f32>,
    pub timeline: Timeline,
    pub attenuation_db: f32,
    pub analysis: Analysis,
    pub loudness: LoudnessReport,
    pub arrangement: ArrangementReport,
    pub transitions: Vec<TransitionReport>,
}

pub fn prepare(project: &Project) -> Result<PreparedAudio> {
    let sr = project.audio.sample_rate;
    let mut clips = Vec::new();
    let mut track_reports = Vec::new();
    for (i, t) in project.tracks.iter().enumerate() {
        let buf: AudioBuffer<f32> = oxiaudio_decode::decode_file(&t.audio)
            .with_context(|| format!("tracks[{i}]: decode {}", t.audio.display()))?;
        let channels = buf.channels.channel_count();
        ensure!(
            channels == 1 || channels == 2,
            "tracks[{i}]: only mono/stereo input is supported"
        );
        ensure!(
            buf.sample_rate > 0 && buf.samples.iter().all(|s| s.is_finite()),
            "tracks[{i}]: invalid sample rate or non-finite PCM"
        );
        let frames = buf.samples.len() / channels;
        // Decoder packet trimming is authoritative. Do not reapply LAME metadata.
        let start = (t.trim_start_seconds * buf.sample_rate as f64).round() as usize;
        let end = t
            .trim_end_seconds
            .map(|v| (v * buf.sample_rate as f64).round() as usize)
            .unwrap_or(frames);
        ensure!(
            start < end && end <= frames,
            "tracks[{i}]: trim range exceeds decoded duration ({:.6} seconds)",
            frames as f64 / buf.sample_rate as f64
        );
        let mut stereo = Vec::with_capacity((end - start) * 2);
        for frame in buf.samples[start * channels..end * channels].chunks_exact(channels) {
            stereo.push(frame[0]);
            stereo.push(frame[channels - 1]);
        }
        let mut stereo = if buf.sample_rate != sr {
            resample(stereo, buf.sample_rate, sr)?
        } else {
            stereo
        };
        let measured = measure(&stereo, sr)?;
        let normalization = if project.audio.loudness.mode == LoudnessMode::Track {
            normalization_gain(&measured, &project.audio.loudness)
        } else {
            0.
        };
        apply_gain(&mut stereo, normalization + t.gain_db as f64);
        track_reports.push(TrackLoudness {
            input: measured,
            normalization_gain_db: normalization,
            manual_gain_db: t.gain_db,
        });
        clips.push(stereo);
    }
    let overlap = if project.audio.transitions.enabled {
        (project.audio.crossfade_seconds * sr as f64).round() as u64
    } else {
        0
    };
    let arrangement_overlap =
        if project.audio.transitions.mode != TransitionMode::Fixed && overlap > 0 {
            0
        } else {
            overlap
        };
    let arrangement =
        arrangement::arrange(&clips, sr, arrangement_overlap, &project.audio.arrangement)?;
    let lengths: Vec<_> = arrangement
        .order
        .iter()
        .map(|&i| clips[i].len() as u64 / 2)
        .collect();
    let transitions = transitions::plan(
        &clips,
        &arrangement.order,
        sr,
        overlap,
        &project.audio.transitions,
        project.audio.crossfade_curve,
        project.audio.crossfade_shape,
    );
    let overlaps: Vec<_> = transitions.iter().map(|t| t.overlap_samples).collect();
    let timeline = Timeline::with_overlaps(&lengths, sr, &overlaps)?;
    let mut pcm = vec![
        0f32;
        usize::try_from(timeline.total_samples)?
            .checked_mul(2)
            .context("PCM size overflow")?
    ];
    for (i, &source) in arrangement.order.iter().enumerate() {
        let clip = &clips[source];
        let span = &timeline.tracks[i];
        let incoming = timeline.incoming_overlap(i);
        let outgoing = timeline.outgoing_overlap(i);
        for (frame, src) in clip.chunks_exact(2).enumerate() {
            let frame = frame as u64;
            let mut gain = 1.;
            if incoming > 0 && frame < incoming {
                if transitions[i - 1].mode == TransitionMode::Smart {
                    continue; // The dynamic boundary is rendered exactly once below.
                }
                gain *= crossfade_gains(
                    frame,
                    incoming,
                    project.audio.crossfade_curve,
                    project.audio.crossfade_shape,
                )
                .1;
            }
            if outgoing > 0 && frame >= span.length - outgoing {
                if transitions[i].mode == TransitionMode::Smart {
                    continue;
                }
                gain *= crossfade_gains(
                    frame - (span.length - outgoing),
                    outgoing,
                    project.audio.crossfade_curve,
                    project.audio.crossfade_shape,
                )
                .0;
            }
            let dst = (span.start + frame) as usize * 2;
            pcm[dst] += src[0] * gain;
            pcm[dst + 1] += src[1] * gain;
        }
    }
    for (i, transition) in transitions.iter().enumerate() {
        if transition.mode != TransitionMode::Smart || transition.overlap_samples == 0 {
            continue;
        }
        let left = &clips[transition.outgoing_source];
        let right = &clips[transition.incoming_source];
        let count = transition.overlap_samples as usize * 2;
        let mixed = transitions::mix_boundary(
            &left[left.len() - count..],
            &right[..count],
            sr,
            transition,
            &project.audio.transitions,
        );
        let start = timeline.tracks[i + 1].start as usize * 2;
        pcm[start..start + count].copy_from_slice(&mixed);
    }
    drop(clips);
    let mix_before = measure(&pcm, sr)?;
    let normalization = if project.audio.loudness.mode == LoudnessMode::Program {
        normalization_gain(&mix_before, &project.audio.loudness)
    } else {
        0.
    };
    let gain = normalization + project.audio.gain_db as f64;
    let attenuation_db = mix_before
        .true_peak_dbtp
        .map(|peak| (project.audio.peak_ceiling_db as f64 - peak - gain).min(0.))
        .unwrap_or(0.);
    apply_gain(&mut pcm, gain + attenuation_db);
    let output = measure(&pcm, sr)?;
    let loudness = LoudnessReport {
        tracks: track_reports,
        mix_before,
        output,
        normalization_gain_db: normalization,
        peak_attenuation_db: attenuation_db,
        target_lufs: (project.audio.loudness.mode != LoudnessMode::Off)
            .then_some(project.audio.loudness.target_lufs),
    };
    let analysis = Analysis::new(&pcm, sr);
    Ok(PreparedAudio {
        pcm,
        timeline,
        attenuation_db: attenuation_db as f32,
        analysis,
        loudness,
        arrangement,
        transitions,
    })
}

pub fn resample(pcm: Vec<f32>, from: u32, to: u32) -> Result<Vec<f32>> {
    let frames = pcm.len() / 2;
    let input = InterleavedOwned::new_from(pcm, 2, frames)?;
    let mut resampler = Fft::<f32>::new(from as usize, to as usize, 1024, 2, FixedSync::Input)?;
    let mut output = resampler.process_all(&input, frames, None)?.take_data();
    // rubato uses floating-point ceil internally, which can add one frame for
    // exact ratios such as 44100 -> 48000. Define duration with integer math.
    let wanted = (frames as u128 * to as u128).div_ceil(from as u128) as usize * 2;
    ensure!(output.len() >= wanted, "resampler returned too few samples");
    output.truncate(wanted);
    Ok(output)
}
fn phase(frame: u64, overlap: u64) -> f32 {
    if overlap <= 1 {
        0.5
    } else {
        frame as f32 / (overlap - 1) as f32
    }
}
pub fn fade_in(frame: u64, overlap: u64) -> f32 {
    (phase(frame, overlap) * std::f32::consts::FRAC_PI_2).sin()
}
pub fn fade_out(frame: u64, overlap: u64) -> f32 {
    (phase(frame, overlap) * std::f32::consts::FRAC_PI_2).cos()
}
/// Gain pairs share the same source-sample clock as the overlap markers.
/// Smooth changes the phase speed while retaining equal-power gain pairs.
pub fn crossfade_gains(frame: u64, overlap: u64, curve: CrossfadeCurve, shape: f32) -> (f32, f32) {
    let mut t = phase(frame, overlap);
    if shape != 1. {
        let a = t.powf(shape);
        let b = (1. - t).powf(shape);
        t = a / (a + b);
    }
    match curve {
        CrossfadeCurve::Linear => (1. - t, t),
        CrossfadeCurve::Smooth => {
            let angle = (t * t * (3. - 2. * t)) * std::f32::consts::FRAC_PI_2;
            (angle.cos(), angle.sin())
        }
        CrossfadeCurve::EqualPower => {
            let angle = t * std::f32::consts::FRAC_PI_2;
            (angle.cos(), angle.sin())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn equal_power_has_exact_endpoints_and_constant_power() {
        assert_eq!(fade_in(0, 100), 0.);
        assert!((fade_out(99, 100)).abs() < 1e-6);
        for i in 0..100 {
            assert!((fade_in(i, 100).powi(2) + fade_out(i, 100).powi(2) - 1.).abs() < 1e-6);
        }
    }
    #[test]
    fn resampling_preserves_length_and_alignment() {
        let mut input = vec![0.; 44100 * 2];
        input[22050 * 2] = 1.;
        input[22050 * 2 + 1] = 1.;
        let out = resample(input, 44100, 48000).unwrap();
        assert_eq!(out.len(), 96000);
        let peak = out
            .chunks_exact(2)
            .enumerate()
            .max_by(|a, b| a.1[0].abs().total_cmp(&b.1[0].abs()))
            .unwrap()
            .0;
        assert!(peak.abs_diff(24000) <= 1, "peak at {peak}");
    }
    #[test]
    fn configurable_fades_have_continuous_endpoints_and_the_expected_power() {
        for shape in [0.25, 1., 4.] {
            for curve in [
                CrossfadeCurve::EqualPower,
                CrossfadeCurve::Smooth,
                CrossfadeCurve::Linear,
            ] {
                let mut last = (1., 0.);
                for frame in 0..1001 {
                    let gain = crossfade_gains(frame, 1001, curve, shape);
                    assert!(gain.0 <= last.0 + 1e-6 && gain.1 >= last.1 - 1e-6);
                    if curve == CrossfadeCurve::Linear {
                        assert!((gain.0 + gain.1 - 1.).abs() < 1e-6);
                    } else {
                        assert!((gain.0 * gain.0 + gain.1 * gain.1 - 1.).abs() < 1e-6);
                    }
                    last = gain;
                }
                assert!(last.0.abs() < 1e-6 && (last.1 - 1.).abs() < 1e-6);
            }
        }
        for frame in 0..100 {
            let (outgoing, incoming) = crossfade_gains(frame, 100, CrossfadeCurve::EqualPower, 1.);
            assert_eq!(outgoing, fade_out(frame, 100));
            assert_eq!(incoming, fade_in(frame, 100));
        }
        let smooth = crossfade_gains(1, 1001, CrossfadeCurve::Smooth, 1.).1;
        let ordinary = crossfade_gains(1, 1001, CrossfadeCurve::EqualPower, 1.).1;
        assert!(smooth < ordinary / 100.);
    }
}
