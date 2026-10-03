//! Original endpoint-aware automix. Analysis is offline; source audio is neither
//! cut nor stretched. Smart mode also shapes local gain, tone and ambience.
use crate::crossfade_gains;
use audorchid_core::{CrossfadeCurve, TransitionMode, Transitions};
use ebur128::{EbuR128, Mode};
use serde::Serialize;

#[derive(Debug, Default, Serialize)]
pub struct TransitionReport {
    pub outgoing_source: usize,
    pub incoming_source: usize,
    pub mode: TransitionMode,
    pub overlap_samples: u64,
    pub overlap_seconds: f64,
    /// Energy-model estimate over 0.5-second windows, not measured output LUFS.
    pub estimated_dip_db: f64,
    pub quiet_tail_seconds: f64,
    pub fade_tail_seconds: f64,
    pub intro_seconds: f64,
    pub outgoing_local_lufs: Option<f64>,
    pub incoming_local_lufs: Option<f64>,
    pub incoming_match_db: f64,
    /// The early-entry requirement could not fit the configured/track limit.
    pub entry_limited: bool,
    pub tone_strength: f64,
    pub reverb_mix: f64,
    #[serde(skip)]
    gains: Vec<[f32; 2]>,
    #[serde(skip)]
    hop: usize,
}

fn smooth(power: &[f64], radius: usize) -> Vec<f64> {
    (0..power.len())
        .map(|i| {
            let slice = &power[i.saturating_sub(radius)..(i + radius + 1).min(power.len())];
            slice.iter().sum::<f64>() / slice.len() as f64
        })
        .collect()
}
fn trailing(power: &[f64], threshold: f64, seconds_per_bin: f64) -> f64 {
    let count = power.iter().rev().take_while(|&&v| v < threshold).count();
    let seconds = count as f64 * seconds_per_bin;
    if seconds < 0.4 { 0. } else { seconds }
}
fn introduction(power: &[f64], threshold: f64, seconds_per_bin: f64) -> f64 {
    // Require a sustained entrance, rather than mistaking one transient for it.
    let window = 5.min(power.len());
    if window == 0 {
        return 0.;
    }
    power
        .windows(window)
        .position(|v| v.iter().sum::<f64>() / window as f64 >= threshold)
        .unwrap_or(power.len()) as f64
        * seconds_per_bin
}
fn local_lufs(pcm: &[f32], sr: u32) -> Option<f64> {
    let mut meter = EbuR128::new(2, sr, Mode::I).ok()?;
    meter.add_frames_f32(pcm).ok()?;
    let lufs = meter.loudness_global().ok()?;
    lufs.is_finite().then_some(lufs)
}
fn ease(t: f64) -> f64 {
    let t = t.clamp(0., 1.);
    t * t * (3. - 2. * t)
}

/// Gain nodes share the 20 Hz analysis clock, with linear interpolation at the
/// sample clock. Boundary nodes are exactly dry/unity to avoid gain steps.
fn smart_gains(
    tail: &[f64],
    head: &[f64],
    overlap: u64,
    (hop, sr): (usize, u32),
    matching_db: f64,
    options: &Transitions,
    (curve, shape): (CrossfadeCurve, f32),
) -> (Vec<[f32; 2]>, f64, f64) {
    if overlap == 0 {
        return (Vec::new(), 0., 0.);
    }
    let bins = (overlap as usize).div_ceil(hop).max(1);
    let tail_start = tail.len().saturating_sub(bins);
    let mean = |values: &[f64]| values.iter().sum::<f64>() / values.len().max(1) as f64;
    let half_second = 10.min(bins);
    let out_start = mean(&tail[tail_start..(tail_start + half_second).min(tail.len())]);
    let in_end =
        mean(&head[bins.saturating_sub(half_second).min(head.len())..bins.min(head.len())]);
    let out_reference = reference(tail);
    let maximum = 10f64.powf(options.max_gain_db / 20.);
    let duration = overlap as f64 / sr as f64;
    let mut gains = Vec::with_capacity(bins + 1);
    let mut output = Vec::with_capacity(bins);
    let mut targets = Vec::with_capacity(bins);
    for i in 0..=bins {
        let frame = (i * hop).min(overlap.saturating_sub(1) as usize) as u64;
        let t = frame as f64 / overlap.saturating_sub(1).max(1) as f64;
        let seconds = frame as f64 / sr as f64;
        let release = ease((duration - seconds) / options.release_seconds);
        let entry = ease(seconds / 2f64.min(duration * 0.3).max(0.05));
        let out_power = tail
            .get(tail_start + i.min(bins - 1))
            .copied()
            .unwrap_or(0.);
        let in_power = head.get(i.min(bins - 1)).copied().unwrap_or(0.);
        let (_, b) = crossfade_gains(frame, overlap, curve, shape);
        let base_weight = (b as f64).powi(2);
        // Advance the entrance when the source already fades; do not wait for
        // the geometric midpoint of a long, nearly silent tail.
        let natural_loss = (1. - out_power / out_reference).clamp(0., 1.);
        let weight = base_weight + (1. - base_weight) * natural_loss * entry;
        let trim = 10f64.powf(matching_db * (1. - ease(t)) * release / 20.);
        let mut a = (1. - weight).max(0.).sqrt();
        let mut b = weight.sqrt() * trim;
        let target = out_start * (1. - ease(t)) + in_end * ease(t);
        let power = out_power * a * a + in_power * b * b;
        // Bound correction and never amplify digital silence. The final true
        // peak check remains authoritative (uncorrelated-power prediction is
        // insufficient for correlated or opposite-phase signals).
        if power > 1e-10 {
            let correction = (target / power).sqrt().clamp(1., maximum);
            let correction = 1. + (correction - 1.) * entry * release;
            a = (a * correction).min(maximum);
            b = (b * correction).min(maximum);
        }
        if i == 0 {
            a = 1.;
            b = 0.;
        }
        if i == bins {
            a = 0.;
            b = 1.;
        }
        gains.push([a as f32, b as f32]);
        if i < bins {
            output.push(out_power * a * a + in_power * b * b);
            targets.push(target);
        }
    }
    let mut total = 0.;
    let mut worst: f64 = 0.;
    for (power, target) in output
        .windows(half_second)
        .zip(targets.windows(half_second))
    {
        let ratio =
            (target.iter().sum::<f64>().max(1e-12) / power.iter().sum::<f64>().max(1e-12)).max(1.);
        let deficit = 10. * ratio.log10();
        total += deficit;
        worst = worst.max(deficit);
    }
    (gains, total / (bins - half_second + 1) as f64, worst)
}

impl TransitionReport {
    fn gains_at(&self, frame: usize) -> [f32; 2] {
        let last = self.overlap_samples.saturating_sub(1) as usize;
        if frame >= last {
            return [0., 1.];
        }
        let node = (frame / self.hop).min(self.gains.len() - 2);
        let start = node * self.hop;
        let end = ((node + 1) * self.hop).min(last);
        let t = (frame - start) as f32 / (end - start).max(1) as f32;
        let a = self.gains[node];
        let b = self.gains[node + 1];
        [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t]
    }
}

/// Small stereo diffusion network, deliberately milder than the reference's
/// noise-IR convolution. Its state and filter history run continuously through
/// each overlap; wet and tone return smoothly to dry at both boundaries.
struct Ambience {
    delays: Vec<Vec<[f32; 2]>>,
    positions: [usize; 4],
    low: [f32; 2],
    high: [f32; 2],
}
impl Ambience {
    fn new(sr: u32, first: [f32; 2]) -> Self {
        Self {
            delays: [0.073, 0.109, 0.151, 0.211]
                .iter()
                .map(|seconds| vec![[0.; 2]; (seconds * sr as f64).round().max(1.) as usize])
                .collect(),
            positions: [0; 4],
            low: first,
            high: first,
        }
    }
    fn frame(&mut self, source: [f32; 2], depth: f32, wet: f32, sr: u32) -> [f32; 2] {
        let low_alpha = 1. - (-std::f32::consts::TAU * 180. / sr as f32).exp();
        let cutoff = (18000. * 0.08f32.powf(depth)).min(sr as f32 * 0.4);
        let high_alpha = 1. - (-std::f32::consts::TAU * cutoff / sr as f32).exp();
        let mut dry = [0.; 2];
        for ch in 0..2 {
            self.low[ch] += low_alpha * (source[ch] - self.low[ch]);
            self.high[ch] += high_alpha * (source[ch] - self.high[ch]);
            // Give overlapping bass and bright transients space without adding
            // low-frequency gain that could consume the whole programme's peak headroom.
            dry[ch] =
                source[ch] * (1. - depth) + self.high[ch] * depth - self.low[ch] * depth * 0.35;
        }
        let mut diffuse = [0.; 2];
        for (i, delay) in self.delays.iter_mut().enumerate() {
            let delayed = delay[self.positions[i]];
            for ch in 0..2 {
                diffuse[ch] += delayed[ch] * 0.25;
                delay[self.positions[i]][ch] = dry[ch] * 0.5 + delayed[1 - ch] * 0.38;
            }
            self.positions[i] = (self.positions[i] + 1) % delay.len();
        }
        [
            dry[0] * (1. - wet) + diffuse[0] * wet,
            dry[1] * (1. - wet) + diffuse[1] * wet,
        ]
    }
}

pub fn mix_boundary(
    left: &[f32],
    right: &[f32],
    sr: u32,
    report: &TransitionReport,
    options: &Transitions,
) -> Vec<f32> {
    let mut outgoing = Ambience::new(sr, [left[0], left[1]]);
    let mut incoming = Ambience::new(sr, [right[0], right[1]]);
    let mut output = Vec::with_capacity(left.len());
    let frames = left.len() / 2;
    for i in 0..frames {
        let t = i as f64 / frames.saturating_sub(1).max(1) as f64;
        let seconds = i as f64 / sr as f64;
        let end_seconds = (frames - 1 - i) as f64 / sr as f64;
        let entry = ease(seconds / options.release_seconds);
        let release = ease(end_seconds / options.release_seconds);
        let out_depth = (options.tone_strength * entry * release * ease(t)) as f32;
        let in_depth = (options.tone_strength * entry * release * (1. - ease(t))) as f32;
        let wet = (options.reverb_mix * entry * release) as f32;
        let a = outgoing.frame([left[i * 2], left[i * 2 + 1]], out_depth, wet, sr);
        let b = incoming.frame([right[i * 2], right[i * 2 + 1]], in_depth, wet, sr);
        let gain = report.gains_at(i);
        output.push(a[0] * gain[0] + b[0] * gain[1]);
        output.push(a[1] * gain[0] + b[1] * gain[1]);
    }
    output
}

fn envelope(pcm: &[f32], hop: usize) -> Vec<f64> {
    pcm.chunks(hop * 2)
        .map(|block| block.iter().map(|v| (*v as f64).powi(2)).sum::<f64>() / block.len() as f64)
        .collect()
}
fn reference(power: &[f64]) -> f64 {
    if power.is_empty() {
        return 1e-12;
    }
    let mut sorted = power.to_vec();
    sorted.sort_by(f64::total_cmp);
    sorted[(sorted.len() - 1) * 3 / 4].max(1e-12)
}
fn dip(
    tail: &[f64],
    head: &[f64],
    overlap: u64,
    hop: usize,
    curve: CrossfadeCurve,
    shape: f32,
) -> (f64, f64) {
    if overlap == 0 {
        return (0., 0.);
    }
    let bins = (overlap as usize).div_ceil(hop).max(1);
    let out_reference = reference(tail);
    let in_reference = reference(head);
    let mut output = Vec::with_capacity(bins);
    let mut target = Vec::with_capacity(bins);
    for i in 0..bins {
        let frame = ((i * hop + hop / 2) as u64).min(overlap.saturating_sub(1));
        let (a, b) = crossfade_gains(frame, overlap, curve, shape);
        let outgoing = tail
            .get(tail.len().saturating_sub(bins) + i)
            .copied()
            .unwrap_or(0.);
        let incoming = head.get(i).copied().unwrap_or(0.);
        output.push(outgoing * (a as f64).powi(2) + incoming * (b as f64).powi(2));
        target.push(out_reference * (a as f64).powi(2) + in_reference * (b as f64).powi(2));
    }
    let window = 10.min(bins); // The envelope clock is 20 Hz: 0.5 seconds.
    let mut total = 0.;
    let mut worst: f64 = 0.;
    for (measured, expected) in output.windows(window).zip(target.windows(window)) {
        let measured = measured.iter().sum::<f64>().max(1e-12);
        let expected = expected.iter().sum::<f64>().max(1e-12);
        let db = (10. * (expected / measured).log10()).max(0.);
        total += db;
        worst = worst.max(db);
    }
    (total / (bins - window + 1) as f64, worst)
}

pub fn plan(
    clips: &[Vec<f32>],
    order: &[usize],
    sr: u32,
    fixed: u64,
    options: &Transitions,
    curve: CrossfadeCurve,
    shape: f32,
) -> Vec<TransitionReport> {
    let fixed = if options.enabled { fixed } else { 0 };
    let hop = (sr as usize / 20).max(1);
    if fixed == 0 {
        return order
            .windows(2)
            .map(|pair| TransitionReport {
                outgoing_source: pair[0],
                incoming_source: pair[1],
                mode: TransitionMode::Fixed,
                hop,
                ..Default::default()
            })
            .collect();
    }
    let scan = (options.analysis_seconds * sr as f64).round() as usize * 2;
    order
        .windows(2)
        .map(|pair| {
            let left = &clips[pair[0]];
            let right = &clips[pair[1]];
            let tail = smooth(&envelope(&left[left.len().saturating_sub(scan)..], hop), 5);
            let head = smooth(&envelope(&right[..right.len().min(scan)], hop), 5);
            let bin_seconds = hop as f64 / sr as f64;
            let quiet_tail_seconds = trailing(
                &tail,
                (reference(&tail) * 10f64.powf(options.tail_threshold_db / 10.)).max(1e-10),
                bin_seconds,
            );
            let fade_tail_seconds = trailing(
                &tail,
                (reference(&tail) * 10f64.powf(options.fade_threshold_db / 10.)).max(1e-10),
                bin_seconds,
            );
            let intro_seconds = introduction(
                &head,
                (reference(&head) * 10f64.powf(options.fade_threshold_db / 10.)).max(1e-10),
                bin_seconds,
            );
            // Local loudness excludes the low-energy tail/intro; it is distinct
            // from the integrated LUFS normalisation of the complete tracks.
            let tail_end = left
                .len()
                .saturating_sub((quiet_tail_seconds * sr as f64).round() as usize * 2);
            let tail_start = tail_end.saturating_sub(sr as usize * 8 * 2);
            let head_start = ((intro_seconds * sr as f64).round() as usize * 2).min(right.len());
            let head_end = (head_start + sr as usize * 8 * 2).min(right.len());
            let outgoing_local_lufs = local_lufs(&left[tail_start..tail_end], sr);
            let incoming_local_lufs = local_lufs(&right[head_start..head_end], sr);
            let incoming_match_db = outgoing_local_lufs
                .zip(incoming_local_lufs)
                .map(|(a, b)| (a - b).clamp(-options.max_gain_db, options.max_gain_db))
                .unwrap_or(0.);
            let adaptive = options.mode == TransitionMode::Adaptive && fixed > 0;
            let smart = options.mode == TransitionMode::Smart && fixed > 0;
            let mut overlap = fixed;
            let mut gains = Vec::new();
            let mut entry_limited = false;
            let mut smart_dip = None;
            if smart {
                let capacity = (left.len().min(right.len()) / 4) as u64;
                let maximum = ((options.max_seconds * sr as f64).round() as u64).min(capacity);
                let required = quiet_tail_seconds + intro_seconds + options.min_seconds;
                let desired =
                    (fade_tail_seconds + intro_seconds + options.min_seconds).max(required);
                entry_limited = required > maximum as f64 / sr as f64;
                let minimum = ((required * sr as f64).round() as u64).min(maximum);
                let step = (sr as u64 / 4).max(1);
                let mut candidates: Vec<_> = (minimum..=maximum).step_by(step as usize).collect();
                if candidates.last() != Some(&maximum) {
                    candidates.push(maximum);
                }
                overlap = candidates
                    .into_iter()
                    .min_by(|&a, &b| {
                        let score = |samples| {
                            let (_, average, worst) = smart_gains(
                                &tail,
                                &head,
                                samples,
                                (hop, sr),
                                incoming_match_db,
                                options,
                                (curve, shape),
                            );
                            let duration = samples as f64 / sr as f64;
                            average
                                + worst * 0.4
                                + duration * 0.12
                                + (desired - duration).max(0.) * 0.4
                        };
                        score(a).total_cmp(&score(b)).then(a.cmp(&b))
                    })
                    .unwrap_or(0);
                let (planned_gains, _, worst) = smart_gains(
                    &tail,
                    &head,
                    overlap,
                    (hop, sr),
                    incoming_match_db,
                    options,
                    (curve, shape),
                );
                gains = planned_gains;
                smart_dip = Some(worst);
            }
            if adaptive {
                // Each neighbour receives at most half a track. Even short middle
                // clips can never participate in a three-track overlap.
                let capacity = (left.len().min(right.len()) / 4) as u64;
                let maximum = ((options.max_seconds * sr as f64).round() as u64).min(capacity);
                let minimum = ((options.min_seconds * sr as f64).round() as u64).min(maximum);
                let step = (sr as u64 / 4).max(1);
                let mut candidates: Vec<_> = (minimum..=maximum).step_by(step as usize).collect();
                if candidates.last() != Some(&maximum) {
                    candidates.push(maximum);
                }
                overlap = candidates
                    .into_iter()
                    .min_by(|&a, &b| {
                        let score = |samples| {
                            let (average, worst) = dip(&tail, &head, samples, hop, curve, shape);
                            // Penalise long simultaneous playback. A longer overlap
                            // must actually reduce a quiet gap to win this comparison.
                            average + worst * 0.35 + 0.18 * samples as f64 / sr as f64
                        };
                        score(a).total_cmp(&score(b)).then(a.cmp(&b))
                    })
                    .unwrap_or(0);
            }
            let (_, estimated_dip_db) = dip(&tail, &head, overlap, hop, curve, shape);
            TransitionReport {
                outgoing_source: pair[0],
                incoming_source: pair[1],
                mode: if smart {
                    TransitionMode::Smart
                } else if adaptive {
                    TransitionMode::Adaptive
                } else {
                    TransitionMode::Fixed
                },
                overlap_samples: overlap,
                overlap_seconds: overlap as f64 / sr as f64,
                estimated_dip_db: smart_dip.unwrap_or(estimated_dip_db),
                quiet_tail_seconds,
                fade_tail_seconds,
                intro_seconds,
                outgoing_local_lufs,
                incoming_local_lufs,
                incoming_match_db,
                entry_limited,
                tone_strength: if smart { options.tone_strength } else { 0. },
                reverb_mix: if smart { options.reverb_mix } else { 0. },
                gains,
                hop,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn smart_detects_early_quiet_tail_and_keeps_sample_boundaries_continuous() {
        let sr = 8000;
        let mut left = vec![0.1; sr as usize * 60 * 2];
        let mut right = left.clone();
        // Last six seconds are genuinely silent; a six-second fade precedes it.
        for i in sr as usize * 48..sr as usize * 60 {
            let gain = ((54. - i as f32 / sr as f32) / 6.).clamp(0., 1.);
            left[i * 2] *= gain;
            left[i * 2 + 1] *= gain;
        }
        for i in 0..sr as usize * 3 {
            right[i * 2] = 0.;
            right[i * 2 + 1] = 0.;
        }
        let options = Transitions::default();
        let clips = vec![left, right];
        let report = plan(
            &clips,
            &[0, 1],
            sr,
            sr as u64 * 2,
            &options,
            CrossfadeCurve::EqualPower,
            1.,
        );
        let r = &report[0];
        assert_eq!(r.mode, TransitionMode::Smart);
        assert!(r.quiet_tail_seconds > 6.);
        assert!(r.fade_tail_seconds > r.quiet_tail_seconds);
        assert!(r.intro_seconds > 2.5);
        assert!(r.overlap_seconds > r.quiet_tail_seconds + r.intro_seconds);
        assert!(!r.entry_limited);
        let n = r.overlap_samples as usize * 2;
        let left = &clips[0][clips[0].len() - n..];
        let right = &clips[1][..n];
        let mixed = mix_boundary(left, right, sr, r, &options);
        assert_eq!(&mixed[..2], &left[..2]);
        assert_eq!(&mixed[n - 2..], &right[n - 2..]);
        assert!(mixed.iter().all(|s| s.is_finite()));
        // DC deliberately exposes discontinuities that a periodic tone can hide.
        let largest_step = mixed
            .chunks_exact(2)
            .map(|f| f[0])
            .collect::<Vec<_>>()
            .windows(2)
            .map(|p| (p[1] - p[0]).abs())
            .fold(0f32, f32::max);
        // The input contains an intentional step at the three-second entrance;
        // outside that single source step the gain/filter clock stays smooth.
        let intro = sr as usize * 3;
        let smooth_step = mixed
            .chunks_exact(2)
            .map(|f| f[0])
            .collect::<Vec<_>>()
            .windows(2)
            .enumerate()
            .filter(|(i, _)| *i != intro - 1)
            .map(|(_, p)| (p[1] - p[0]).abs())
            .fold(0f32, f32::max);
        assert!(
            smooth_step < 0.002,
            "step={smooth_step}, source step={largest_step}"
        );
        let maximum = 10f32.powf(options.max_gain_db as f32 / 20.);
        assert!(
            r.gains
                .iter()
                .flatten()
                .all(|&g| g >= 0. && g <= maximum + 1e-6)
        );
    }
    #[test]
    fn smart_silence_limits_and_disabled_effects_are_safe() {
        let sr = 8000;
        let clips = vec![vec![0.; sr as usize * 4 * 2]; 3];
        let options = Transitions {
            tone_strength: 0.,
            reverb_mix: 0.,
            ..Default::default()
        };
        let reports = plan(
            &clips,
            &[0, 1, 2],
            sr,
            16000,
            &options,
            CrossfadeCurve::Smooth,
            1.,
        );
        assert!(reports[0].entry_limited);
        assert!(
            reports[0].overlap_samples + reports[1].overlap_samples <= clips[1].len() as u64 / 2
        );
        let count = reports[0].overlap_samples as usize * 2;
        let mixed = mix_boundary(
            &clips[0][..count],
            &clips[1][..count],
            sr,
            &reports[0],
            &options,
        );
        assert!(mixed.iter().all(|&s| s == 0.));
        let gapless = plan(
            &clips,
            &[0, 1, 2],
            sr,
            0,
            &options,
            CrossfadeCurve::Smooth,
            1.,
        );
        assert!(
            gapless
                .iter()
                .all(|r| r.mode == TransitionMode::Fixed && r.overlap_samples == 0)
        );
        let disabled = Transitions {
            enabled: false,
            ..options
        };
        let plain = plan(
            &clips,
            &[0, 1, 2],
            sr,
            16000,
            &disabled,
            CrossfadeCurve::Smooth,
            1.,
        );
        assert!(
            plain
                .iter()
                .all(|r| r.overlap_samples == 0 && r.gains.is_empty())
        );
    }
    #[test]
    fn adaptive_bridges_existing_fades_without_changing_source_lengths() {
        let sr = 100;
        let mut left = vec![0.5; 20 * sr as usize * 2];
        let mut right = left.clone();
        for i in 0..600 {
            let gain = (1. - i as f32 / 599.).powi(2);
            let pos = left.len() - 1200 + i * 2;
            left[pos] *= gain;
            left[pos + 1] *= gain;
        }
        for i in 0..300 {
            right[i * 2] *= i as f32 / 299.;
            right[i * 2 + 1] *= i as f32 / 299.;
        }
        let clips = vec![left, right];
        let fixed = plan(
            &clips,
            &[0, 1],
            sr,
            200,
            &Transitions {
                mode: TransitionMode::Fixed,
                ..Default::default()
            },
            CrossfadeCurve::EqualPower,
            1.,
        );
        let options = Transitions {
            mode: TransitionMode::Adaptive,
            ..Default::default()
        };
        let adaptive = plan(
            &clips,
            &[0, 1],
            sr,
            200,
            &options,
            CrossfadeCurve::EqualPower,
            1.,
        );
        assert!(adaptive[0].overlap_samples > 200);
        assert!(adaptive[0].estimated_dip_db < fixed[0].estimated_dip_db);
        assert_eq!(clips[0].len(), 4000);
        let gapless = plan(
            &clips,
            &[0, 1],
            sr,
            0,
            &options,
            CrossfadeCurve::EqualPower,
            1.,
        );
        assert_eq!(gapless[0].overlap_samples, 0);
        let short = vec![vec![0.5; 400], vec![0.5; 400], vec![0.5; 400]];
        let planned = plan(
            &short,
            &[0, 1, 2],
            sr,
            50,
            &options,
            CrossfadeCurve::EqualPower,
            1.,
        );
        assert!(planned[0].overlap_samples + planned[1].overlap_samples <= 200);
    }
}
