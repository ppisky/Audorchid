use crate::ProgressScope;
use anyhow::{Context, Result, ensure};
use serde::Serialize;

#[derive(Debug, Default)]
pub struct ProgressState {
    pub elapsed: f64,
    pub duration: f64,
    pub fraction: f32,
    pub overlaps: Vec<[f32; 2]>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TrackSpan {
    pub start: u64,
    pub length: u64,
}
impl TrackSpan {
    pub fn end(&self) -> u64 {
        self.start + self.length
    }
}
#[derive(Debug, Clone, Serialize)]
pub struct Timeline {
    pub sample_rate: u32,
    /// Common duration for fixed overlaps; zero when durations vary. Use overlaps
    /// or incoming_overlap() for timing, rather than this compatibility field.
    pub overlap: u64,
    pub overlaps: Vec<u64>,
    pub tracks: Vec<TrackSpan>,
    pub total_samples: u64,
}
impl Timeline {
    /// Progress follows the actual decoded source position, including the part
    /// already heard during the incoming fade. It is never a combined two-song bar.
    pub fn progress(&self, sample: u64, scope: ProgressScope) -> ProgressState {
        let Some(active) = self.active_track(sample) else {
            return ProgressState::default();
        };
        let span = &self.tracks[active];
        let (start, length) = match scope {
            ProgressScope::Track => (span.start, span.length),
            ProgressScope::Program => (0, self.total_samples),
        };
        let elapsed = sample.saturating_sub(start).min(length);
        let mut overlaps = Vec::new();
        for (i, next) in self.tracks.iter().enumerate().skip(1) {
            let overlap = self.incoming_overlap(i);
            if overlap > 0 {
                let left = next.start.max(start);
                let right = (next.start + overlap).min(start + length);
                if left < right {
                    overlaps.push([
                        (left - start) as f32 / length as f32,
                        (right - start) as f32 / length as f32,
                    ]);
                }
            }
        }
        ProgressState {
            elapsed: elapsed as f64 / self.sample_rate as f64,
            duration: length as f64 / self.sample_rate as f64,
            fraction: elapsed as f32 / length as f32,
            overlaps,
        }
    }
    pub fn new(lengths: &[u64], sample_rate: u32, overlap: u64) -> Result<Self> {
        let mut timeline = Self::with_overlaps(
            lengths,
            sample_rate,
            &vec![overlap; lengths.len().saturating_sub(1)],
        )?;
        timeline.overlap = overlap;
        Ok(timeline)
    }
    pub fn incoming_overlap(&self, track: usize) -> u64 {
        track
            .checked_sub(1)
            .and_then(|i| self.overlaps.get(i))
            .copied()
            .unwrap_or(0)
    }
    pub fn outgoing_overlap(&self, track: usize) -> u64 {
        self.overlaps.get(track).copied().unwrap_or(0)
    }
    pub fn with_overlaps(lengths: &[u64], sample_rate: u32, overlaps: &[u64]) -> Result<Self> {
        ensure!(
            !lengths.is_empty() && sample_rate > 0,
            "empty timeline or zero sample rate"
        );
        ensure!(
            overlaps.len() == lengths.len() - 1,
            "one overlap is required per track boundary"
        );
        let mut tracks = Vec::new();
        let mut start = 0u64;
        for (i, &length) in lengths.iter().enumerate() {
            ensure!(length > 0, "tracks[{i}] has no samples after trimming");
            let incoming = i
                .checked_sub(1)
                .and_then(|j| overlaps.get(j))
                .copied()
                .unwrap_or(0);
            let outgoing = overlaps.get(i).copied().unwrap_or(0);
            let occupied = incoming.checked_add(outgoing).context("overlap overflow")?;
            ensure!(
                length >= occupied && length > incoming.max(outgoing),
                "tracks[{i}] is too short for the requested crossfade (overlap must fit each neighbour and must not create a three-track overlap)"
            );
            tracks.push(TrackSpan { start, length });
            start = start
                .checked_add(length)
                .ok_or_else(|| anyhow::anyhow!("timeline overflow"))?;
            if i + 1 < lengths.len() {
                start -= outgoing;
            }
        }
        Ok(Self {
            sample_rate,
            overlap: overlaps
                .first()
                .copied()
                .filter(|first| overlaps.iter().all(|v| v == first))
                .unwrap_or(0),
            overlaps: overlaps.to_vec(),
            tracks,
            total_samples: start,
        })
    }
    pub fn frames(&self, num: u32, den: u32) -> u64 {
        (self.total_samples as u128 * num as u128).div_ceil(self.sample_rate as u128 * den as u128)
            as u64
    }
    pub fn sample_at_frame(&self, k: u64, num: u32, den: u32) -> u64 {
        (k as u128 * self.sample_rate as u128 * den as u128 / num as u128) as u64
    }
    /// Main track changes at the overlap midpoint, independently of video speed.
    pub fn active_track(&self, sample: u64) -> Option<usize> {
        if sample >= self.total_samples {
            return None;
        }
        self.tracks
            .iter()
            .enumerate()
            .rev()
            .find(|(i, s)| sample >= s.start + self.incoming_overlap(*i) / 2)
            .map(|(i, _)| i)
    }
    /// Visual chapters use the SAME midpoint as titles and chapter counters.
    pub fn chapter_starts(&self) -> Vec<u64> {
        self.tracks
            .iter()
            .enumerate()
            .map(|(i, s)| s.start + self.incoming_overlap(i) / 2)
            .collect()
    }
    pub fn seconds(&self) -> f64 {
        self.total_samples as f64 / self.sample_rate as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn variable_overlaps_share_the_audio_metadata_and_progress_clock() {
        let t = Timeline::with_overlaps(&[1000, 1000, 1000], 100, &[400, 200]).unwrap();
        assert_eq!(t.total_samples, 2400);
        assert_eq!(t.chapter_starts(), vec![0, 800, 1500]);
        assert_eq!(t.active_track(799), Some(0));
        assert_eq!(t.active_track(800), Some(1));
        assert_eq!(t.active_track(1500), Some(2));
        assert_eq!(
            t.progress(800, ProgressScope::Track).overlaps,
            vec![[0., 0.4], [0.8, 1.]]
        );
        assert!(Timeline::with_overlaps(&[1000, 500, 1000], 100, &[400, 200]).is_err());
    }
    #[test]
    fn rational_clock_has_no_accumulated_drift() {
        let t = Timeline::new(&[48_000 * 60 * 60], 48000, 0).unwrap();
        assert_eq!(t.frames(30000, 1001), 107893);
        assert_eq!(t.sample_at_frame(30000, 30000, 1001), 48_048_000);
    }
    #[test]
    fn overlap_shortens_timeline_and_handoff_is_midpoint() {
        let t = Timeline::new(&[100, 100], 100, 20).unwrap();
        assert_eq!(t.total_samples, 180);
        assert_eq!(t.active_track(89), Some(0));
        assert_eq!(t.active_track(90), Some(1));
        assert_eq!(t.chapter_starts(), vec![0, 90]);
        assert_eq!(t.active_track(180), None);
        assert!(Timeline::new(&[100, 30, 100], 100, 20).is_err());
    }
    #[test]
    fn song_progress_switches_with_metadata_and_marks_only_real_overlaps() {
        let t = Timeline::new(&[1600, 1600], 100, 200).unwrap();
        let before = t.progress(1499, ProgressScope::Track);
        assert_eq!(before.duration, 16.);
        assert_eq!(before.overlaps, vec![[0.875, 1.]]);
        let after = t.progress(1500, ProgressScope::Track);
        assert_eq!(after.elapsed, 1.);
        assert_eq!(after.overlaps, vec![[0., 0.125]]);
        let program = t.progress(1500, ProgressScope::Program);
        assert_eq!(program.duration, 30.);
        assert_eq!(program.overlaps, vec![[1400. / 3000., 1600. / 3000.]]);
        assert!(t.progress(3000, ProgressScope::Track).overlaps.is_empty());
        let gapless = Timeline::new(&[1600, 1600], 100, 0).unwrap();
        assert!(
            gapless
                .progress(1500, ProgressScope::Track)
                .overlaps
                .is_empty()
        );
    }
}
