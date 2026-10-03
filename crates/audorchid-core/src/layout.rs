use crate::{Layout, Preset, ProgressPosition, Project, VisualizerPosition};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};

/// Rectangles are [left, top, width, height], in pixels of the output canvas.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Geometry {
    pub cover: [f32; 4],
    pub title: [f32; 4],
    pub artist: [f32; 4],
    pub waveform: [f32; 4],
    pub progress: [f32; 4],
    pub time: [f32; 4],
    pub title_size: f32,
    pub artist_size: f32,
    pub time_size: f32,
}
impl Project {
    pub fn geometry(&self) -> Result<Geometry> {
        let w = self.video.width;
        let h = self.video.height;
        let (rw, rh, mut g) = match self.layout.preset {
            Preset::Landscape => (
                1920,
                1080,
                Geometry {
                    cover: [130., 170., 620., 620.],
                    title: [870., 340., 920., 84.],
                    artist: [870., 438., 920., 46.],
                    waveform: [130., 900., 1660., 130.],
                    progress: [130., 865., 1660., 6.],
                    time: [130., 810., 1660., 40.],
                    title_size: 70.,
                    artist_size: 36.,
                    time_size: 26.,
                },
            ),
            Preset::Portrait => (
                1080,
                1920,
                Geometry {
                    cover: [120., 240., 840., 840.],
                    title: [120., 1140., 840., 77.],
                    artist: [120., 1230., 840., 43.],
                    waveform: [120., 1630., 840., 220.],
                    progress: [120., 1570., 840., 6.],
                    time: [120., 1520., 840., 40.],
                    title_size: 64.,
                    artist_size: 34.,
                    time_size: 26.,
                },
            ),
            Preset::Square => (
                1080,
                1080,
                Geometry {
                    cover: [300., 80., 480., 480.],
                    title: [90., 610., 900., 58.],
                    artist: [90., 678., 900., 36.],
                    waveform: [90., 880., 900., 140.],
                    progress: [90., 850., 900., 5.],
                    time: [90., 800., 900., 36.],
                    title_size: 48.,
                    artist_size: 28.,
                    time_size: 22.,
                },
            ),
            Preset::PortraitThreeFour => (
                1080,
                1440,
                Geometry {
                    cover: [220., 120., 640., 640.],
                    title: [90., 812., 900., 65.],
                    artist: [90., 888., 900., 39.],
                    waveform: [90., 1190., 900., 190.],
                    progress: [90., 1155., 900., 5.],
                    time: [90., 1105., 900., 36.],
                    title_size: 54.,
                    artist_size: 30.,
                    time_size: 24.,
                },
            ),
            Preset::Custom => {
                let mut g = self.layout.custom.clone().ok_or_else(|| {
                    anyhow::anyhow!(
                        "layout.custom: define all rectangles and font sizes when preset = 'custom'"
                    )
                })?;
                g.arrange(&self.layout, w.min(h) as f32 / 1080.);
                g.validate(w, h)?;
                return Ok(g);
            }
        };
        ensure!(
            self.layout.custom.is_none(),
            "layout.custom is only allowed when layout.preset = 'custom'"
        );
        ensure!(
            w as u64 * rh as u64 == h as u64 * rw as u64,
            "video.width/height does not match layout.preset; use {rw}x{rh}, an exact scaled size, or preset = 'custom'"
        );
        let scale = w as f32 / rw as f32;
        for r in [
            &mut g.cover,
            &mut g.title,
            &mut g.artist,
            &mut g.waveform,
            &mut g.progress,
            &mut g.time,
        ] {
            for v in r {
                *v *= scale;
            }
        }
        g.title_size *= scale;
        g.artist_size *= scale;
        g.time_size *= scale;
        g.arrange(&self.layout, w.min(h) as f32 / 1080.);
        g.validate(w, h)?;
        Ok(g)
    }
}
impl Geometry {
    fn arrange(&mut self, layout: &Layout, pixel_scale: f32) {
        let position = match layout.progress_position {
            ProgressPosition::Auto if layout.preset == Preset::Custom => ProgressPosition::Above,
            ProgressPosition::Auto => ProgressPosition::UnderArtistLower,
            value => value,
        };
        let unit = self.artist_size / 36.;
        if layout.visualizer_position == VisualizerPosition::UnderArtist {
            self.waveform = [
                self.artist[0],
                self.artist[1] + self.artist[3] + 124. * unit,
                self.artist[2],
                120. * unit,
            ];
        }
        // Shrink from the baseline, keeping a stable left edge for alignment.
        let height = self.waveform[3] * layout.visualizer_scale[1];
        self.waveform[1] += self.waveform[3] - height;
        self.waveform[2] *= layout.visualizer_scale[0];
        self.waveform[3] = height;
        match position {
            ProgressPosition::UnderArtist | ProgressPosition::UnderArtistLower => {
                self.progress = [
                    self.artist[0],
                    self.artist[1] + self.artist[3] + 32. * unit,
                    self.artist[2] * 0.84,
                    4. * unit,
                ];
                if position == ProgressPosition::UnderArtistLower {
                    // Same metadata grid as under_artist; only move vertically.
                    self.progress[1] += 72. * unit;
                }
                self.time = [
                    self.progress[0],
                    self.progress[1] + 20. * unit,
                    self.progress[2],
                    30. * unit,
                ];
                self.time_size = 22. * unit;
                if layout.visualizer_position == VisualizerPosition::UnderArtist {
                    self.waveform[1] =
                        self.waveform[1].max(self.time[1] + self.time[3] + 42. * unit);
                }
            }
            ProgressPosition::Attached => {
                self.progress[0] = self.waveform[0];
                self.progress[1] = self.waveform[1] + self.waveform[3];
                self.progress[2] = self.waveform[2];
            }
            ProgressPosition::Above | ProgressPosition::Auto => {}
        }
        if layout.visualizer_position == VisualizerPosition::UnderArtist
            && !matches!(
                position,
                ProgressPosition::UnderArtist | ProgressPosition::UnderArtistLower
            )
        {
            if position == ProgressPosition::Above {
                self.progress = [
                    self.waveform[0],
                    self.waveform[1] - 20. * unit,
                    self.waveform[2],
                    4. * unit,
                ];
            }
            self.time = [
                self.waveform[0],
                self.waveform[1] - 62. * unit,
                self.waveform[2],
                30. * unit,
            ];
            self.time_size = 22. * unit;
        }
        self.progress[2] *= layout.progress_scale[0];
        self.progress[3] *= layout.progress_scale[1];
        self.time[0] = self.progress[0];
        self.time[2] = self.progress[2];
        for rect in [&mut self.progress, &mut self.time] {
            rect[0] += layout.progress_offset[0] * pixel_scale;
            rect[1] += layout.progress_offset[1] * pixel_scale;
        }
    }
    fn validate(&self, w: u32, h: u32) -> Result<()> {
        for (name, r) in [
            ("cover", self.cover),
            ("title", self.title),
            ("artist", self.artist),
            ("waveform", self.waveform),
            ("progress", self.progress),
            ("time", self.time),
        ] {
            ensure!(
                r.iter().all(|x| x.is_finite())
                    && r[0] >= 0.
                    && r[1] >= 0.
                    && r[2] > 0.
                    && r[3] > 0.
                    && r[0] + r[2] <= w as f32 + 0.01
                    && r[1] + r[3] <= h as f32 + 0.01,
                "layout.{name}: rectangle must fit inside the {w}x{h} canvas"
            );
        }
        for (name, s, r) in [
            ("title", self.title_size, self.title),
            ("artist", self.artist_size, self.artist),
            ("time", self.time_size, self.time),
        ] {
            ensure!(
                s.is_finite() && s > 0. && s <= r[3],
                "layout.{name}_size: must be positive and fit the text box height"
            );
        }
        Ok(())
    }
}
