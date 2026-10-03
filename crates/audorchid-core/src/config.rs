use crate::styles::*;
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Project {
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,
    #[serde(default = "default_name")]
    pub name: String,
    #[serde(default)]
    pub output: Output,
    #[serde(default)]
    pub video: Video,
    #[serde(default)]
    pub audio: Audio,
    #[serde(default)]
    pub layout: Layout,
    #[serde(default)]
    pub visual: Visual,
    #[serde(default)]
    pub fonts: Fonts,
    pub tracks: Vec<Track>,
}
fn default_name() -> String {
    "Audorchid".into()
}
fn default_schema_version() -> u32 {
    1
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct Output {
    pub path: PathBuf,
    #[serde(default)]
    pub overwrite: bool,
}
impl Default for Output {
    fn default() -> Self {
        Self {
            path: "audorchid.mp4".into(),
            overwrite: false,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct Video {
    pub width: u32,
    pub height: u32,
    pub fps_num: u32,
    pub fps_den: u32,
    pub bitrate_kbps: u32,
    pub keyframe_interval: u32,
    pub rate_control: VideoRateControl,
    pub quality_qp: u8,
}
impl Default for Video {
    fn default() -> Self {
        Self {
            width: 1920,
            height: 1080,
            fps_num: 30,
            fps_den: 1,
            bitrate_kbps: 8000,
            keyframe_interval: 60,
            rate_control: VideoRateControl::Quality,
            quality_qp: 18,
        }
    }
}
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct Audio {
    pub sample_rate: u32,
    pub bitrate_kbps: u32,
    pub crossfade_seconds: f64,
    pub crossfade_curve: CrossfadeCurve,
    pub crossfade_shape: f32,
    pub gain_db: f32,
    pub peak_ceiling_db: f32,
    pub loudness: Loudness,
    pub arrangement: Arrangement,
    pub transitions: Transitions,
}
impl Default for Audio {
    fn default() -> Self {
        Self {
            sample_rate: 48000,
            bitrate_kbps: 192,
            crossfade_seconds: 2.0,
            crossfade_curve: CrossfadeCurve::EqualPower,
            crossfade_shape: 1.0,
            gain_db: 0.0,
            peak_ceiling_db: -1.0,
            loudness: Loudness::default(),
            arrangement: Arrangement::default(),
            transitions: Transitions::default(),
        }
    }
}
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct Fonts {
    /// Omitted paths use embedded fonts; explicit paths must exist.
    pub latin: Option<PathBuf>,
    pub cjk: Option<PathBuf>,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct Visual {
    pub waveform: bool,
    pub spectrum: bool,
    pub triangles: u32,
    pub seed: u64,
    pub triangle_style: TriangleStyle,
    pub spectrum_style: SpectrumStyle,
    pub waveform_style: WaveformStyle,
    pub progress_style: ProgressStyle,
    pub artwork_style: ArtworkStyle,
}
impl Default for Visual {
    fn default() -> Self {
        Self {
            waveform: false,
            spectrum: true,
            triangles: 0,
            seed: 673742,
            triangle_style: TriangleStyle::default(),
            spectrum_style: SpectrumStyle::default(),
            waveform_style: WaveformStyle::default(),
            progress_style: ProgressStyle::default(),
            artwork_style: ArtworkStyle::default(),
        }
    }
}
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Track {
    pub audio: PathBuf,
    pub cover: Option<PathBuf>,
    /// Optional variation for generated artwork when no cover path is provided.
    pub cover_seed: Option<u64>,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub artist: String,
    #[serde(default)]
    pub trim_start_seconds: f64,
    pub trim_end_seconds: Option<f64>,
    #[serde(default)]
    pub gain_db: f32,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct Layout {
    pub preset: Preset,
    pub custom: Option<crate::Geometry>,
    pub progress_position: ProgressPosition,
    pub visualizer_position: VisualizerPosition,
    pub visualizer_scale: [f32; 2],
    pub progress_scale: [f32; 2],
    pub progress_offset: [f32; 2],
}
impl Default for Layout {
    fn default() -> Self {
        Self {
            preset: Preset::Landscape,
            custom: None,
            progress_position: ProgressPosition::Auto,
            visualizer_position: VisualizerPosition::Bottom,
            visualizer_scale: [1., 1.],
            progress_scale: [1., 1.],
            progress_offset: [0., 0.],
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub enum Preset {
    #[serde(rename = "16:9")]
    Landscape,
    #[serde(rename = "9:16")]
    Portrait,
    #[serde(rename = "1:1")]
    Square,
    #[serde(rename = "3:4")]
    PortraitThreeFour,
    #[serde(rename = "custom")]
    Custom,
}

impl Project {
    /// Resolve omitted dimensions from the preset and omitted titles from filenames.
    /// Unknown fields remain errors even in a minimal configuration.
    pub fn from_toml(text: &str) -> Result<Self> {
        let document: toml::Table = toml::from_str(text).context("invalid TOML configuration")?;
        let video = document.get("video").and_then(toml::Value::as_table);
        let has_width = video.is_some_and(|v| v.contains_key("width"));
        let has_height = video.is_some_and(|v| v.contains_key("height"));
        let mut p: Self = document.try_into().context("invalid TOML configuration")?;
        if p.layout.preset != Preset::Custom {
            let (width, height) = match p.layout.preset {
                Preset::Landscape => (1920u64, 1080u64),
                Preset::Portrait => (1080, 1920),
                Preset::Square => (1080, 1080),
                Preset::PortraitThreeFour => (1080, 1440),
                Preset::Custom => unreachable!(),
            };
            match (has_width, has_height) {
                (false, false) => {
                    p.video.width = width as u32;
                    p.video.height = height as u32;
                }
                (true, false) => {
                    let n = p.video.width as u64 * height;
                    ensure!(
                        n.is_multiple_of(width),
                        "video.width cannot form this exact preset ratio"
                    );
                    p.video.height = (n / width).try_into()?;
                }
                (false, true) => {
                    let n = p.video.height as u64 * width;
                    ensure!(
                        n.is_multiple_of(height),
                        "video.height cannot form this exact preset ratio"
                    );
                    p.video.width = (n / height).try_into()?;
                }
                _ => {}
            }
        }
        for track in &mut p.tracks {
            if track.title.trim().is_empty() {
                track.title = track
                    .audio
                    .file_stem()
                    .context("audio path has no filename")?
                    .to_string_lossy()
                    .into_owned();
            }
        }
        p.validate()?;
        Ok(p)
    }

    pub fn resolved_spectrum_style(&self) -> Result<SpectrumStyle> {
        let mut style = self.visual.spectrum_style.clone();
        if style.bars.is_none() {
            let scale = self.video.width.min(self.video.height) as f32 / 1080.;
            let width = self.geometry()?.waveform[2] / scale;
            let pitch = style.bar_width / (1. - style.gap);
            style.bars = Some(((width / pitch).floor() as usize).clamp(8, 256));
        }
        Ok(style)
    }

    /// Relative paths always resolve against the TOML, never the process cwd.
    pub fn load(path: &Path) -> Result<Self> {
        let absolute = path
            .canonicalize()
            .with_context(|| format!("config: {}", path.display()))?;
        let text = std::fs::read_to_string(&absolute)?;
        let mut p = Self::from_toml(&text)?;
        let base = absolute.parent().unwrap();
        let resolve = |p: &mut PathBuf| {
            if p.is_relative() {
                *p = base.join(&*p);
            }
        };
        resolve(&mut p.output.path);
        if let Some(p) = &mut p.fonts.latin {
            resolve(p);
        }
        if let Some(p) = &mut p.fonts.cjk {
            resolve(p);
        }
        for track in &mut p.tracks {
            resolve(&mut track.audio);
            if let Some(p) = &mut track.cover {
                resolve(p);
            }
        }
        Ok(p)
    }
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.schema_version == 1,
            "schema_version: supported version is 1"
        );
        ensure!(
            !self.tracks.is_empty(),
            "tracks: at least one [[tracks]] section is required"
        );
        ensure!(
            self.output
                .path
                .extension()
                .is_some_and(|s| s.eq_ignore_ascii_case("mp4")),
            "output.path must end in .mp4"
        );
        for (key, n) in [("width", self.video.width), ("height", self.video.height)] {
            ensure!(
                (32..=4096).contains(&n) && n % 2 == 0,
                "video.{key}: must be an even integer between 32 and 4096"
            );
        }
        ensure!(
            self.video.fps_num > 0 && self.video.fps_den > 0,
            "video: fps_num and fps_den must be positive"
        );
        let fps = self.video.fps_num as f64 / self.video.fps_den as f64;
        ensure!(
            (1.0..=120.0).contains(&fps),
            "video: supported frame rate is 1..120 fps"
        );
        ensure!(
            (100..=100_000).contains(&self.video.bitrate_kbps),
            "video.bitrate_kbps: expected 100..100000"
        );
        ensure!(
            self.video.keyframe_interval > 0,
            "video.keyframe_interval must be positive"
        );
        ensure!(
            [44100, 48000].contains(&self.audio.sample_rate),
            "audio.sample_rate: use 44100 or 48000"
        );
        ensure!(
            (32..=320).contains(&self.audio.bitrate_kbps),
            "audio.bitrate_kbps: stereo AAC-LC supports 32..320"
        );
        ensure!(
            self.audio.crossfade_seconds.is_finite()
                && (0.0..=30.0).contains(&self.audio.crossfade_seconds),
            "audio.crossfade_seconds: expected 0..30 seconds (0 = gapless concatenation)"
        );
        ensure!(
            self.audio.gain_db.is_finite() && (-60.0..=24.0).contains(&self.audio.gain_db),
            "audio.gain_db: expected -60..24 dB"
        );
        ensure!(
            self.audio.peak_ceiling_db.is_finite()
                && (-24.0..=0.0).contains(&self.audio.peak_ceiling_db),
            "audio.peak_ceiling_db: expected -24..0 dBFS"
        );
        ensure!(
            self.visual.triangles <= 256,
            "visual.triangles: maximum is 256"
        );
        for (i, t) in self.tracks.iter().enumerate() {
            ensure!(!t.title.trim().is_empty(), "tracks[{i}].title is empty");
            ensure!(
                t.trim_start_seconds.is_finite() && t.trim_start_seconds >= 0.0,
                "tracks[{i}].trim_start_seconds must be finite and nonnegative"
            );
            if let Some(end) = t.trim_end_seconds {
                ensure!(
                    end.is_finite() && end > t.trim_start_seconds,
                    "tracks[{i}].trim_end_seconds must be after the start"
                );
            }
            ensure!(
                t.gain_db.is_finite() && (-60.0..=24.0).contains(&t.gain_db),
                "tracks[{i}].gain_db: expected -60..24 dB"
            );
        }
        self.validate_styles()?;
        self.geometry()?;
        Ok(())
    }
    pub fn check_files(&self) -> Result<()> {
        let mut files: Vec<_> = self.fonts.latin.iter().collect();
        files.extend(self.fonts.cjk.iter());
        for t in &self.tracks {
            files.push(&t.audio);
            files.extend(t.cover.iter());
        }
        for path in files {
            ensure!(path.is_file(), "missing input file: {}", path.display());
        }
        Ok(())
    }
    pub fn check_output(&self) -> Result<()> {
        if self.output.path.exists() {
            ensure!(
                self.output.overwrite,
                "output exists: {} (set output.overwrite = true to replace it)",
                self.output.path.display()
            );
            let out = self.output.path.canonicalize()?;
            ensure!(
                !self
                    .tracks
                    .iter()
                    .any(|t| t.audio.canonicalize().ok().as_ref() == Some(&out)
                        || t.cover
                            .as_ref()
                            .and_then(|p| p.canonicalize().ok())
                            .as_ref()
                            == Some(&out)),
                "output would overwrite an input"
            );
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn example() -> Project {
        toml::from_str(include_str!("../../../examples/audorchid.en.toml")).unwrap()
    }
    #[test]
    fn examples_are_equivalent_and_valid() {
        let en = example();
        let zh: Project =
            toml::from_str(include_str!("../../../examples/audorchid.zh-CN.toml")).unwrap();
        en.validate().unwrap();
        zh.validate().unwrap();
        assert_eq!(
            serde_json::to_value(en).unwrap(),
            serde_json::to_value(zh).unwrap()
        );
        let en =
            Project::from_toml(include_str!("../../../examples/audorchid.simple.en.toml")).unwrap();
        let zh = Project::from_toml(include_str!(
            "../../../examples/audorchid.simple.zh-CN.toml"
        ))
        .unwrap();
        assert_eq!(
            serde_json::to_value(en).unwrap(),
            serde_json::to_value(zh).unwrap()
        );
    }
    #[test]
    fn minimal_config_infers_titles_fonts_dimensions_and_audio_defaults() {
        let source = "[[tracks]]\naudio = '音乐/晨光.wav'\n[[tracks]]\naudio = 'second.flac'\n";
        let p = Project::from_toml(source).unwrap();
        assert_eq!(p.schema_version, 1);
        assert_eq!(p.tracks[0].title, "晨光");
        assert_eq!(p.tracks[1].title, "second");
        assert!(p.fonts.latin.is_none() && p.fonts.cjk.is_none());
        assert_eq!(p.output.path, Path::new("audorchid.mp4"));
        assert_eq!(p.audio.arrangement.mode, ArrangementMode::Automatic);
        assert_eq!(p.audio.loudness.mode, LoudnessMode::Track);
        assert_eq!(p.audio.transitions.mode, crate::TransitionMode::Smart);
        assert!(p.audio.transitions.enabled && p.audio.arrangement.enabled);
        let disabled = Project::from_toml(&format!(
            "[audio.arrangement]\nenabled = false\n[audio.transitions]\nenabled = false\n{source}"
        ))
        .unwrap();
        assert!(!disabled.audio.arrangement.enabled && !disabled.audio.transitions.enabled);
        assert_eq!(p.visual.progress_style.color, "#FFFFFF");
        assert_eq!(p.visual.artwork_style.corner_radius, 24.);
        for (preset, width, height) in [
            ("16:9", 1920, 1080),
            ("9:16", 1080, 1920),
            ("1:1", 1080, 1080),
            ("3:4", 1080, 1440),
        ] {
            let p =
                Project::from_toml(&format!("[layout]\npreset = '{preset}'\n{source}")).unwrap();
            assert_eq!((p.video.width, p.video.height), (width, height));
        }
        let p = Project::from_toml(&format!(
            "[layout]\npreset = '9:16'\n[video]\nheight = 960\n{source}"
        ))
        .unwrap();
        assert_eq!((p.video.width, p.video.height), (540, 960));
        assert!(Project::from_toml(&format!("[video]\nwidth = 321\n{source}")).is_err());
        let p = Project::from_toml(&format!(
            "[audio.arrangement]\nmode = 'configuration'\n[audio.loudness]\nmode = 'off'\n{source}"
        ))
        .unwrap();
        assert_eq!(p.audio.arrangement.mode, ArrangementMode::Configuration);
        assert_eq!(p.audio.loudness.mode, LoudnessMode::Off);
        assert!(Project::from_toml("[[tracks]]\naudio = 'a.wav'\nunknown = true").is_err());
    }
    #[test]
    fn adaptive_bands_keep_density_when_layout_changes_but_not_when_resolution_changes() {
        let mut p = example();
        p.visual.spectrum_style.bars = None;
        for (preset, width, height, bars) in [
            (Preset::Landscape, 1920, 1080, 80),
            (Preset::Portrait, 1080, 1920, 40),
            (Preset::Square, 1080, 1080, 43),
            (Preset::PortraitThreeFour, 1080, 1440, 43),
        ] {
            p.layout.preset = preset;
            p.video.width = width;
            p.video.height = height;
            assert_eq!(p.resolved_spectrum_style().unwrap().bars, Some(bars));
            p.video.width /= 2;
            p.video.height /= 2;
            assert_eq!(p.resolved_spectrum_style().unwrap().bars, Some(bars));
        }
        p.layout.visualizer_scale = [0.5, 1.];
        assert_eq!(p.resolved_spectrum_style().unwrap().bars, Some(21));
        p.visual.spectrum_style.bars = Some(64);
        assert_eq!(p.resolved_spectrum_style().unwrap().bars, Some(64));
        p.visual.spectrum_style.bars = None;
        p.visual.spectrum_style.bar_width = 24.;
        assert!(p.resolved_spectrum_style().unwrap().band_count() < 21);
    }
    #[test]
    fn preset_ratios_are_enforced_and_positions_scale() {
        let mut p = example();
        let full = p.geometry().unwrap();
        p.video.width = 960;
        p.video.height = 540;
        let half = p.geometry().unwrap();
        assert_eq!(half.cover[0], full.cover[0] / 2.);
        assert_eq!(half.title_size, full.title_size / 2.);
        p.layout.preset = Preset::Portrait;
        assert!(p.validate().is_err());
        p.video.width = 1080;
        p.video.height = 1920;
        p.validate().unwrap();
        p.layout.preset = Preset::Square;
        p.video.height = 1080;
        p.validate().unwrap();
        p.layout.preset = Preset::PortraitThreeFour;
        p.video.height = 1440;
        p.validate().unwrap();
        p.layout.progress_position = ProgressPosition::Attached;
        let g = p.geometry().unwrap();
        assert_eq!(g.progress[0], g.waveform[0]);
        assert_eq!(g.progress[1], g.waveform[1] + g.waveform[3]);
        assert_eq!(g.progress[2], g.waveform[2]);
    }
    #[test]
    fn custom_is_explicit_and_rejects_out_of_canvas_coordinates() {
        let mut p = example();
        let geometry = p.geometry().unwrap();
        p.layout.preset = Preset::Custom;
        assert!(p.validate().is_err());
        p.layout.custom = Some(geometry);
        p.validate().unwrap();
        p.layout.custom.as_mut().unwrap().title[0] = 1900.;
        assert!(p.validate().is_err());
    }
    #[test]
    fn unknown_options_and_nonfinite_numbers_fail() {
        let source = include_str!("../../../examples/audorchid.en.toml");
        assert!(
            toml::from_str::<Project>(&source.replace("fps_num = 30", "fps_typo = 30")).is_err()
        );
        let mut p = example();
        p.audio.crossfade_seconds = f64::NAN;
        assert!(p.validate().is_err());
        p = example();
        p.video.width = 1919;
        assert!(p.validate().is_err());
        p = example();
        p.visual.triangle_style.opacity = f32::NAN;
        assert!(p.validate().is_err());
        p = example();
        p.visual.spectrum_style.bars = Some(0);
        assert!(p.validate().is_err());
        p = example();
        p.audio.loudness.target_lufs = f64::INFINITY;
        assert!(p.validate().is_err());
        assert!(crate::rgb_hex("#😀ff").is_err());
    }
    #[test]
    fn compact_and_docked_layouts_fit_every_preset() {
        let mut p = example();
        for (preset, width, height) in [
            (Preset::Landscape, 1920, 1080),
            (Preset::Portrait, 1080, 1920),
            (Preset::Square, 1080, 1080),
            (Preset::PortraitThreeFour, 1080, 1440),
        ] {
            p.layout.preset = preset;
            p.video.width = width;
            p.video.height = height;
            for position in [VisualizerPosition::Bottom, VisualizerPosition::UnderArtist] {
                p.layout.visualizer_position = position;
                for progress in [
                    ProgressPosition::UnderArtist,
                    ProgressPosition::UnderArtistLower,
                    ProgressPosition::Above,
                    ProgressPosition::Attached,
                ] {
                    p.layout.progress_position = progress;
                    p.validate().unwrap();
                    let g = p.geometry().unwrap();
                    assert!(g.artist[1] >= g.title[1] + g.title[3]);
                    assert!(g.artist[1] - (g.title[1] + g.title[3]) <= g.title_size * 0.25);
                    if progress == ProgressPosition::UnderArtist {
                        assert_eq!(g.artist[0], g.progress[0]);
                        assert!(g.progress[2] < g.artist[2]);
                        assert!(g.progress[1] > g.artist[1] + g.artist[3]);
                    }
                    if progress == ProgressPosition::UnderArtistLower {
                        assert!(g.progress[1] > g.artist[1] + g.artist[3]);
                        if preset == Preset::Landscape {
                            assert!(g.progress[0] > g.cover[0] + g.cover[2]);
                            assert_eq!(g.progress[0], g.artist[0]);
                        }
                    }
                }
            }
        }
        let legacy = include_str!("../../../examples/audorchid.en.toml").replace(
            "[visual.spectrum_style]",
            "[visual.spectrum_style]\npalette = \"classic\"",
        );
        assert!(toml::from_str::<Project>(&legacy).is_err());
        p.layout.visualizer_scale = [f32::NAN, 1.];
        assert!(p.validate().is_err());
        let mut p = example();
        let original = p.geometry().unwrap();
        p.layout.progress_offset = [-24., 30.];
        let moved = p.geometry().unwrap();
        assert_eq!(moved.progress[0], original.progress[0] - 24.);
        assert_eq!(moved.time[1], original.time[1] + 30.);
        p.layout.progress_offset = [4096., 0.];
        assert!(p.validate().is_err());
    }
}
