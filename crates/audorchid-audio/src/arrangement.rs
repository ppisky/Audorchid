//! Deterministic offline sequencing from decoded audio, without changing speed
//! or trimming material. Endpoint analysis and crossfade duration are separate.
use anyhow::{Result, ensure};
use audorchid_core::{Arrangement, ArrangementMode};
use rustfft::{FftPlanner, num_complex::Complex32};
use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub struct Endpoint {
    pub rms_db: f64,
    pub centroid_hz: f64,
    /// Heuristic beat estimate. None means insufficient rhythmic evidence.
    pub tempo_bpm: Option<f64>,
}
#[derive(Clone, Debug, Serialize)]
pub struct TrackFeatures {
    pub head: Endpoint,
    pub tail: Endpoint,
}
#[derive(Debug, Serialize)]
pub struct ArrangementReport {
    pub mode: ArrangementMode,
    /// Zero-based indices into the TOML tracks; timeline spans use this order.
    pub order: Vec<usize>,
    /// Features stay in configuration order, as do the per-track LUFS reports.
    pub features: Vec<TrackFeatures>,
    pub analysis_seconds: f64,
}

fn endpoint(pcm: &[f32], sr: u32) -> Endpoint {
    let frames = pcm.len() / 2;
    let mean_square =
        pcm.iter().map(|v| (*v as f64).powi(2)).sum::<f64>() / pcm.len().max(1) as f64;
    let rms_db = (10. * mean_square.max(1e-12).log10()).max(-120.);
    // Keep stereo powers separate: opposite-phase channels must not cancel.
    let n = 4096;
    let fft = FftPlanner::<f32>::new().plan_fft_forward(n);
    let mut buffer = vec![Complex32::default(); n];
    let mut weighted = 0.;
    let mut power = 0.;
    for start in (0..frames).step_by((sr as usize / 10).max(1)) {
        for channel in 0..2 {
            for (i, bin) in buffer.iter_mut().enumerate() {
                let value = pcm.get((start + i) * 2 + channel).copied().unwrap_or(0.);
                let hann = 0.5 - 0.5 * (std::f32::consts::TAU * i as f32 / n as f32).cos();
                *bin = Complex32::new(value * hann, 0.);
            }
            fft.process(&mut buffer);
            for (k, bin) in buffer.iter().enumerate().take(n / 2).skip(1) {
                let hz = k as f64 * sr as f64 / n as f64;
                if !(30. ..=16000.).contains(&hz) {
                    continue;
                }
                let magnitude = bin.norm() as f64;
                weighted += hz * magnitude;
                power += magnitude;
            }
        }
    }
    let hop = (sr as usize / 100).max(1);
    let envelope: Vec<f64> = pcm
        .chunks(hop * 2)
        .map(|chunk| {
            (chunk.iter().map(|v| (*v as f64).powi(2)).sum::<f64>() / chunk.len() as f64).sqrt()
        })
        .collect();
    Endpoint {
        rms_db,
        centroid_hz: if power > 1e-9 { weighted / power } else { 0. },
        tempo_bpm: estimate_tempo(&envelope, sr as f64 / hop as f64),
    }
}

fn estimate_tempo(envelope: &[f64], hz: f64) -> Option<f64> {
    if envelope.len() < (hz * 4.) as usize {
        return None;
    }
    let mean = envelope.iter().sum::<f64>() / envelope.len() as f64;
    let variation =
        (envelope.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / envelope.len() as f64).sqrt();
    if mean < 1e-6 || variation < mean * 0.08 {
        return None;
    }
    let onset: Vec<_> = envelope.windows(2).map(|w| (w[1] - w[0]).max(0.)).collect();
    let mut best = (0., 0.);
    for lag in (hz * 60. / 180.).round() as usize..=(hz * 60. / 60.).round() as usize {
        if lag == 0 || lag >= onset.len() / 2 {
            continue;
        }
        let mut dot = 0.;
        let mut aa = 0.;
        let mut bb = 0.;
        for i in lag..onset.len() {
            dot += onset[i] * onset[i - lag];
            aa += onset[i].powi(2);
            bb += onset[i - lag].powi(2);
        }
        let score = dot / (aa * bb).sqrt().max(1e-20);
        if score > best.0 {
            best = (score, 60. * hz / lag as f64);
        }
    }
    (best.0 >= 0.25).then_some(best.1)
}

fn tempo_distance(a: Option<f64>, b: Option<f64>) -> f64 {
    match (a, b) {
        (Some(a), Some(b)) => [0.5, 1., 2.]
            .into_iter()
            .map(|factor| (a / (b * factor)).log2().abs())
            .fold(f64::INFINITY, f64::min)
            .min(1.),
        _ => 0.,
    }
}
fn target(curve: &[f64], position: usize, count: usize) -> f64 {
    let x = position as f64 / count.saturating_sub(1).max(1) as f64 * (curve.len() - 1) as f64;
    let left = x.floor() as usize;
    let right = (left + 1).min(curve.len() - 1);
    curve[left] + (curve[right] - curve[left]) * (x - left as f64)
}
fn sequence(
    features: &[TrackFeatures],
    lengths: &[u64],
    overlap: u64,
    style: &Arrangement,
) -> Result<Vec<usize>> {
    let energy: Vec<_> = features
        .iter()
        .map(|f| (f.head.rms_db + f.tail.rms_db) / 2.)
        .collect();
    let lo = energy.iter().copied().fold(f64::INFINITY, f64::min);
    let hi = energy.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let normalized: Vec<_> = energy
        .iter()
        .map(|v| {
            if hi - lo < 0.01 {
                0.5
            } else {
                (v - lo) / (hi - lo)
            }
        })
        .collect();
    let mut order: Vec<usize> = Vec::with_capacity(features.len());
    let mut remaining: Vec<_> = (0..features.len()).collect();
    while !remaining.is_empty() {
        let position = order.len();
        let candidate = remaining
            .iter()
            .copied()
            .filter(|&i| {
                if style.keep_first && position == 0 && i != 0 {
                    return false;
                }
                // Short tracks can occupy either endpoint, but never two overlaps.
                if position > 0 && remaining.len() > 1 && lengths[i] < 2 * overlap {
                    return false;
                }
                let short_left = remaining
                    .iter()
                    .filter(|&&j| j != i && lengths[j] < 2 * overlap)
                    .count();
                short_left <= usize::from(remaining.len() > 1)
            })
            .map(|i| {
                let f = &features[i];
                let mut cost = style.energy_weight
                    * (normalized[i] - target(&style.energy_curve, position, features.len())).abs();
                if let Some(&previous) = order.last() {
                    let tail = &features[previous].tail;
                    cost += style.transition_weight
                        * ((tail.rms_db - f.head.rms_db).abs() / 24.).min(1.);
                    cost += style.brightness_weight
                        * ((tail.centroid_hz.max(30.) / f.head.centroid_hz.max(30.))
                            .log2()
                            .abs()
                            / 8.)
                            .min(1.);
                    cost += style.tempo_weight * tempo_distance(tail.tempo_bpm, f.head.tempo_bpm);
                }
                (i, cost)
            })
            .min_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)));
        let Some((chosen, _)) = candidate else {
            anyhow::bail!(
                "automatic arrangement cannot fit crossfades: shorten audio.crossfade_seconds or disable keep_first"
            );
        };
        order.push(chosen);
        remaining.retain(|&i| i != chosen);
    }
    Ok(order)
}

pub fn arrange(
    clips: &[Vec<f32>],
    sr: u32,
    overlap: u64,
    style: &Arrangement,
) -> Result<ArrangementReport> {
    let mut report = ArrangementReport {
        mode: if style.enabled {
            style.mode
        } else {
            ArrangementMode::Configuration
        },
        order: (0..clips.len()).collect(),
        features: Vec::new(),
        analysis_seconds: style.analysis_seconds,
    };
    if report.mode == ArrangementMode::Configuration {
        return Ok(report);
    }
    let window = (style.analysis_seconds * sr as f64).round() as usize * 2;
    let lengths: Vec<_> = clips.iter().map(|v| v.len() as u64 / 2).collect();
    ensure!(
        clips.len() == 1 || lengths.iter().all(|&n| n > overlap),
        "a track is shorter than its crossfade"
    );
    for clip in clips {
        let size = window.min(clip.len());
        report.features.push(TrackFeatures {
            head: endpoint(&clip[..size], sr),
            tail: endpoint(&clip[clip.len() - size..], sr),
        });
    }
    report.order = sequence(&report.features, &lengths, overlap, style)?;
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn configuration_order_is_exact_and_automatic_is_deterministic() {
        let clips = vec![vec![0.; 200]; 3];
        assert_eq!(
            arrange(&clips, 100, 0, &Arrangement::default())
                .unwrap()
                .order,
            vec![0, 1, 2]
        );
        let features: Vec<_> = [-10., -40., -25.]
            .into_iter()
            .map(|rms_db| {
                let f = Endpoint {
                    rms_db,
                    centroid_hz: 1000.,
                    tempo_bpm: Some(120.),
                };
                TrackFeatures {
                    head: f.clone(),
                    tail: f,
                }
            })
            .collect();
        let mut style = Arrangement {
            energy_curve: vec![0., 1.],
            transition_weight: 0.,
            ..Default::default()
        };
        assert_eq!(
            sequence(&features, &[100; 3], 0, &style).unwrap(),
            vec![1, 2, 0]
        );
        let disabled = Arrangement {
            enabled: false,
            ..style.clone()
        };
        let report = arrange(&clips, 100, 0, &disabled).unwrap();
        assert_eq!(report.mode, ArrangementMode::Configuration);
        assert_eq!(report.order, vec![0, 1, 2]);
        assert!(report.features.is_empty());
        style.keep_first = true;
        assert_eq!(sequence(&features, &[100; 3], 0, &style).unwrap()[0], 0);
        style.keep_first = false;
        let result = sequence(&features, &[100, 30, 30], 20, &style).unwrap();
        assert_eq!(result[1], 0); // only the long clip fits both fades
    }
    #[test]
    fn endpoint_analysis_detects_pulse_tempo_and_preserves_antiphase_power() {
        let sr = 8000;
        let pcm: Vec<f32> = (0..sr * 8)
            .flat_map(|i| {
                let value = if i % (sr / 2) < sr / 12 {
                    (i as f32 * 1000. * std::f32::consts::TAU / sr as f32).sin()
                        * 0.4
                        * (std::f32::consts::PI * (i % (sr / 2)) as f32 / (sr / 12) as f32)
                            .sin()
                            .powi(2)
                } else {
                    0.
                };
                [value, -value]
            })
            .collect();
        let f = endpoint(&pcm, sr);
        assert!(f.rms_db > -30.);
        assert!((f.centroid_hz - 1000.).abs() < 150.);
        assert!((f.tempo_bpm.unwrap() - 120.).abs() < 2.);
        assert!(
            endpoint(&vec![0.; sr as usize * 2 * 5], sr)
                .tempo_bpm
                .is_none()
        );
    }
}
