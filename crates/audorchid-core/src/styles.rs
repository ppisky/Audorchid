use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LoudnessMode {
    Off,
    #[default]
    Track,
    Program,
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CrossfadeCurve {
    #[default]
    EqualPower,
    Smooth,
    Linear,
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TransitionMode {
    Fixed,
    Adaptive,
    #[default]
    Smart,
}
/// Endpoint-energy planning, separate from playlist ordering and beat matching.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct Transitions {
    pub enabled: bool,
    pub mode: TransitionMode,
    pub min_seconds: f64,
    pub max_seconds: f64,
    pub analysis_seconds: f64,
    pub tail_threshold_db: f64,
    pub fade_threshold_db: f64,
    pub max_gain_db: f64,
    pub tone_strength: f64,
    pub reverb_mix: f64,
    pub release_seconds: f64,
}
impl Default for Transitions {
    fn default() -> Self {
        Self {
            enabled: true,
            mode: TransitionMode::Smart,
            min_seconds: 4.,
            max_seconds: 24.,
            analysis_seconds: 40.,
            tail_threshold_db: -24.,
            fade_threshold_db: -6.,
            max_gain_db: 3.,
            tone_strength: 0.35,
            reverb_mix: 0.08,
            release_seconds: 1.,
        }
    }
}
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct Loudness {
    pub mode: LoudnessMode,
    pub target_lufs: f64,
    pub max_gain_db: f64,
}
impl Default for Loudness {
    fn default() -> Self {
        Self {
            mode: LoudnessMode::Track,
            target_lufs: -16.,
            max_gain_db: 12.,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProgressPosition {
    #[default]
    Auto,
    UnderArtist,
    #[serde(alias = "cover_side")]
    UnderArtistLower,
    Above,
    Attached,
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProgressScope {
    #[default]
    Track,
    Program,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct ProgressStyle {
    pub scope: ProgressScope,
    pub color: String,
    pub track_color: String,
    pub track_opacity: f32,
    pub overlap_color: String,
    pub overlap_opacity: f32,
    pub show_overlap: bool,
    pub corner_radius: f32,
    pub show_counter: bool,
    pub counter_right_inset: f32,
}
impl Default for ProgressStyle {
    fn default() -> Self {
        Self {
            scope: ProgressScope::Track,
            color: "#FFFFFF".into(),
            track_color: "#FFFFFF".into(),
            track_opacity: 0.15,
            overlap_color: "#EAC768".into(),
            overlap_opacity: 0.8,
            show_overlap: true,
            corner_radius: 2.,
            show_counter: true,
            counter_right_inset: 0.,
        }
    }
}
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct ArtworkStyle {
    pub corner_radius: f32,
    pub border_width: f32,
    /// Gaussian sigma at the 96x96 background working resolution; cover remains sharp.
    pub background_blur: f32,
}
impl Default for ArtworkStyle {
    fn default() -> Self {
        Self {
            corner_radius: 24.,
            border_width: 3.,
            background_blur: 18.,
        }
    }
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum VideoRateControl {
    #[default]
    Quality,
    Bitrate,
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ArrangementMode {
    Configuration,
    #[default]
    Automatic,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct Arrangement {
    pub enabled: bool,
    pub mode: ArrangementMode,
    pub analysis_seconds: f64,
    pub keep_first: bool,
    pub energy_curve: Vec<f64>,
    pub energy_weight: f64,
    pub tempo_weight: f64,
    pub brightness_weight: f64,
    pub transition_weight: f64,
}
impl Default for Arrangement {
    fn default() -> Self {
        Self {
            enabled: true,
            mode: ArrangementMode::Automatic,
            analysis_seconds: 15.,
            keep_first: false,
            energy_curve: vec![0.2, 0.65, 1., 0.4],
            energy_weight: 1.,
            tempo_weight: 0.35,
            brightness_weight: 0.25,
            transition_weight: 0.75,
        }
    }
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum VisualizerPosition {
    #[default]
    Bottom,
    UnderArtist,
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TriangleMode {
    #[default]
    Auto,
    Filled,
    Outline,
    Both,
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TrianglePreset {
    #[default]
    Layered,
    Border,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct TriangleStyle {
    pub preset: TrianglePreset,
    pub mode: TriangleMode,
    pub color: String,
    pub color_light: String,
    pub opacity: f32,
    pub min_size: f32,
    pub max_size: f32,
    pub speed: f32,
    pub stroke_width: f32,
    pub music_reactivity: f32,
    pub size: f32,
    pub thickness: f32,
}
impl Default for TriangleStyle {
    fn default() -> Self {
        Self {
            preset: TrianglePreset::Layered,
            mode: TriangleMode::Auto,
            color: "#080A10".into(),
            color_light: "#343943".into(),
            opacity: 0.28,
            min_size: 100.,
            max_size: 360.,
            speed: 24.,
            stroke_width: 1.5,
            music_reactivity: 0.15,
            size: 120.,
            thickness: 0.04,
        }
    }
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SpectrumPalette {
    #[default]
    White,
    Solid,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct SpectrumStyle {
    pub corner_radius: f32,
    pub palette: SpectrumPalette,
    pub color: String,
    /// None adapts to geometry. An explicit count overrides the width target.
    pub bars: Option<usize>,
    /// Target bar width at a 1080-pixel short edge, excluding its gap.
    pub bar_width: f32,
    pub gap: f32,
    pub peak_caps: bool,
    pub floor_db: f32,
    pub release_ms: f32,
    pub peak_hold_ms: f32,
    pub peak_fall_ms: f32,
}
impl Default for SpectrumStyle {
    fn default() -> Self {
        Self {
            corner_radius: 0.,
            palette: SpectrumPalette::Solid,
            color: "#FFFFFF".into(),
            bars: None,
            bar_width: 14.5,
            gap: 0.3,
            peak_caps: false,
            floor_db: -60.,
            release_ms: 180.,
            peak_hold_ms: 250.,
            peak_fall_ms: 800.,
        }
    }
}
impl SpectrumStyle {
    /// Standalone audio analysis has no layout; the render pipeline resolves
    /// adaptive counts before analysis. Its reference layout has 80 bands.
    pub fn band_count(&self) -> usize {
        self.bars.unwrap_or(80)
    }
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WaveformMode {
    Envelope,
    #[default]
    Scope,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct WaveformStyle {
    pub mode: WaveformMode,
    pub color: String,
    pub window_ms: f32,
    pub gain: f32,
    pub line_width: f32,
}
impl Default for WaveformStyle {
    fn default() -> Self {
        Self {
            mode: WaveformMode::Scope,
            color: "#FFFFFF".into(),
            window_ms: 40.,
            gain: 1.5,
            line_width: 1.5,
        }
    }
}
pub fn rgb_hex(s: &str) -> Result<u32> {
    ensure!(
        s.len() == 7 && s.starts_with('#') && s[1..].bytes().all(|c| c.is_ascii_hexdigit()),
        "color must be #RRGGBB"
    );
    Ok(u32::from_str_radix(&s[1..], 16)?)
}
pub(crate) fn finite(name: &str, value: f64, min: f64, max: f64) -> Result<()> {
    ensure!(
        value.is_finite() && (min..=max).contains(&value),
        "{name}: expected {min}..{max}"
    );
    Ok(())
}
impl crate::Project {
    pub(crate) fn validate_styles(&self) -> Result<()> {
        finite(
            "audio.crossfade_shape",
            self.audio.crossfade_shape as f64,
            0.25,
            4.,
        )?;
        let transition = &self.audio.transitions;
        finite(
            "audio.transitions.min_seconds",
            transition.min_seconds,
            0.25,
            60.,
        )?;
        finite(
            "audio.transitions.max_seconds",
            transition.max_seconds,
            transition.min_seconds,
            60.,
        )?;
        finite(
            "audio.transitions.analysis_seconds",
            transition.analysis_seconds,
            transition.max_seconds,
            120.,
        )?;
        finite(
            "audio.transitions.tail_threshold_db",
            transition.tail_threshold_db,
            -60.,
            -12.,
        )?;
        finite(
            "audio.transitions.fade_threshold_db",
            transition.fade_threshold_db,
            -12.,
            -1.,
        )?;
        finite(
            "audio.transitions.max_gain_db",
            transition.max_gain_db,
            0.,
            6.,
        )?;
        finite(
            "audio.transitions.tone_strength",
            transition.tone_strength,
            0.,
            1.,
        )?;
        finite(
            "audio.transitions.reverb_mix",
            transition.reverb_mix,
            0.,
            0.25,
        )?;
        finite(
            "audio.transitions.release_seconds",
            transition.release_seconds,
            0.1,
            4.,
        )?;
        ensure!(
            self.video.quality_qp <= 51,
            "video.quality_qp: expected 0..51"
        );
        let a = &self.audio.arrangement;
        finite(
            "audio.arrangement.analysis_seconds",
            a.analysis_seconds,
            1.,
            60.,
        )?;
        ensure!(
            (2..=32).contains(&a.energy_curve.len()),
            "energy_curve needs 2..32 points"
        );
        for &v in &a.energy_curve {
            finite("energy_curve", v, 0., 1.)?;
        }
        for v in [
            a.energy_weight,
            a.tempo_weight,
            a.brightness_weight,
            a.transition_weight,
        ] {
            finite("arrangement weight", v, 0., 10.)?;
        }
        ensure!(
            a.energy_weight + a.tempo_weight + a.brightness_weight + a.transition_weight > 0.,
            "at least one arrangement weight must be positive"
        );
        let p = &self.visual.progress_style;
        for c in [&p.color, &p.track_color, &p.overlap_color] {
            rgb_hex(c)?;
        }
        finite("progress track_opacity", p.track_opacity as f64, 0., 1.)?;
        finite("progress overlap_opacity", p.overlap_opacity as f64, 0., 1.)?;
        finite("progress corner_radius", p.corner_radius as f64, 0., 100.)?;
        finite(
            "progress counter_right_inset",
            p.counter_right_inset as f64,
            0.,
            100.,
        )?;
        let art = &self.visual.artwork_style;
        finite("artwork corner_radius", art.corner_radius as f64, 0., 1080.)?;
        finite("artwork border_width", art.border_width as f64, 0., 20.)?;
        finite(
            "artwork background_blur",
            art.background_blur as f64,
            0.,
            48.,
        )?;
        finite(
            "spectrum corner_radius",
            self.visual.spectrum_style.corner_radius as f64,
            0.,
            100.,
        )?;
        for (name, factors) in [
            ("layout.visualizer_scale", self.layout.visualizer_scale),
            ("layout.progress_scale", self.layout.progress_scale),
        ] {
            for factor in factors {
                finite(name, factor as f64, 0.25, 1.)?;
            }
        }
        let l = &self.audio.loudness;
        for offset in self.layout.progress_offset {
            finite("layout.progress_offset", offset as f64, -4096., 4096.)?;
        }
        finite("audio.loudness.target_lufs", l.target_lufs, -36., -5.)?;
        finite("audio.loudness.max_gain_db", l.max_gain_db, 0., 24.)?;
        let t = &self.visual.triangle_style;
        rgb_hex(&t.color)?;
        rgb_hex(&t.color_light)?;
        finite("triangle size", t.size as f64, 8., 2160.)?;
        finite("triangle thickness", t.thickness as f64, 0., 1.)?;
        finite("triangle opacity", t.opacity as f64, 0., 1.)?;
        finite("triangle min_size", t.min_size as f64, 8., 1080.)?;
        finite(
            "triangle max_size",
            t.max_size as f64,
            t.min_size as f64,
            2160.,
        )?;
        finite("triangle speed", t.speed as f64, 0., 500.)?;
        finite("triangle stroke_width", t.stroke_width as f64, 0.1, 20.)?;
        finite(
            "triangle music_reactivity",
            t.music_reactivity as f64,
            0.,
            1.,
        )?;
        let s = &self.visual.spectrum_style;
        rgb_hex(&s.color)?;
        if let Some(bars) = s.bars {
            ensure!((8..=256).contains(&bars), "spectrum bars: expected 8..256");
        }
        finite("spectrum bar_width", s.bar_width as f64, 2., 100.)?;
        finite("spectrum gap", s.gap as f64, 0., 0.9)?;
        finite("spectrum floor_db", s.floor_db as f64, -100., -12.)?;
        finite("spectrum release_ms", s.release_ms as f64, 0., 2000.)?;
        finite("spectrum peak_hold_ms", s.peak_hold_ms as f64, 0., 2000.)?;
        finite("spectrum peak_fall_ms", s.peak_fall_ms as f64, 10., 5000.)?;
        let w = &self.visual.waveform_style;
        rgb_hex(&w.color)?;
        finite("waveform window_ms", w.window_ms as f64, 5., 200.)?;
        finite("waveform gain", w.gain as f64, 0.1, 10.)?;
        finite("waveform line_width", w.line_width as f64, 0.1, 10.)?;
        Ok(())
    }
}
