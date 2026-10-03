use anyhow::{Result, ensure};
use audorchid_core::Project;
use clap::{Args, Parser, Subcommand};
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(
    name = "audorchid",
    version,
    about = "Audorchid / 音兰 — render music to H.264 + AAC MP4"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Args)]
pub struct OutputOptions {
    /// Temporary output path, relative to the terminal directory. Does not edit TOML.
    #[arg(short, long)]
    pub output: Option<PathBuf>,
    /// Replace an existing output deliberately. Source files are always protected.
    #[arg(long)]
    pub overwrite: bool,
    /// Temporary resolution factor (0 < scale <= 1), retaining the exact aspect ratio.
    #[arg(long, default_value_t = 1.)]
    pub scale: f64,
}

impl Default for OutputOptions {
    fn default() -> Self {
        Self {
            output: None,
            overwrite: false,
            scale: 1.,
        }
    }
}

#[derive(Subcommand)]
pub enum Command {
    /// Validate configuration, input files, fonts and decoded timeline.
    Check { config: PathBuf },
    /// Print the decoded render plan as JSON (no GPU needed).
    Inspect { config: PathBuf },
    /// Render the complete configured playlist, without a window or audio device.
    Render {
        config: PathBuf,
        #[command(flatten)]
        options: OutputOptions,
    },
    /// Save the first frame (or --seconds) to PNG, without encoding a video.
    #[command(name = "preview", visible_alias = "frame")]
    Frame {
        config: PathBuf,
        #[arg(long, default_value_t = 0.)]
        seconds: f64,
        #[command(flatten)]
        options: OutputOptions,
    },
    /// Save a short WAV excerpt of the actual prepared mix, without a GPU or video encoding.
    AudioPreview {
        config: PathBuf,
        #[arg(long, default_value_t = 0.)]
        seconds: f64,
        #[arg(long, default_value_t = 15.)]
        duration: f64,
        #[arg(short, long)]
        output: PathBuf,
        #[arg(long)]
        overwrite: bool,
    },
    /// Preview one actual playlist handoff, with context before and after it.
    TransitionPreview {
        config: PathBuf,
        /// One-based handoff number AFTER playlist arrangement (1 = first to second).
        #[arg(long, default_value = "1")]
        transition: std::num::NonZeroUsize,
        #[arg(long, default_value_t = 6.)]
        before: f64,
        #[arg(long, default_value_t = 6.)]
        after: f64,
        /// Defaults to <configured-output>.transition-<number>.wav beside the video.
        #[arg(short, long)]
        output: Option<PathBuf>,
        #[arg(long)]
        overwrite: bool,
    },
    /// Generate an original ring/gradient PNG cover; no project or music is needed.
    Cover {
        #[arg(short, long)]
        output: PathBuf,
        #[arg(long, default_value_t = 1600)]
        size: u32,
        #[arg(long, default_value_t = 0)]
        seed: u64,
        #[arg(long)]
        overwrite: bool,
    },
    /// Create original sampled music, MIDI scores and PNG covers; never overwrites existing files.
    Demo {
        #[arg(long, default_value = "examples/media")]
        dir: PathBuf,
    },
}

pub fn save_png(image: &image::RgbaImage, output: &Path, overwrite: bool) -> Result<()> {
    let parent = output
        .parent()
        .ok_or_else(|| anyhow::anyhow!("PNG output has no parent"))?;
    std::fs::create_dir_all(parent)?;
    let mut temp = tempfile::Builder::new()
        .prefix(".audorchid-preview-")
        .suffix(".png.part")
        .tempfile_in(parent)?;
    image.write_to(temp.as_file_mut(), image::ImageFormat::Png)?;
    temp.as_file().sync_all()?;
    if overwrite {
        temp.persist(output)?;
    } else {
        temp.persist_noclobber(output)?;
    }
    Ok(())
}

pub fn terminal_path(path: &Path) -> Result<PathBuf> {
    Ok(if path.is_absolute() {
        path.to_owned()
    } else {
        std::env::current_dir()?.join(path)
    })
}

pub fn preview_path(project: &Project, options: &OutputOptions) -> Result<PathBuf> {
    let path = match &options.output {
        Some(path) => terminal_path(path)?,
        None => project.output.path.with_extension("preview.png"),
    };
    ensure!(
        path.extension()
            .is_some_and(|s| s.eq_ignore_ascii_case("png")),
        "preview output must end in .png"
    );
    Ok(path)
}

pub fn transition_path(
    project: &Project,
    output: Option<&Path>,
    transition: usize,
) -> Result<PathBuf> {
    match output {
        Some(path) => terminal_path(path),
        None => {
            let stem = project
                .output
                .path
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy();
            Ok(project
                .output
                .path
                .with_file_name(format!("{stem}.transition-{transition:02}.wav")))
        }
    }
}

/// Only export operations use CLI paths. TOML inputs always keep their own base.
pub fn apply_options(project: &mut Project, options: &OutputOptions, video: bool) -> Result<()> {
    if video {
        if let Some(path) = &options.output {
            project.output.path = terminal_path(path)?;
        }
        project.output.overwrite |= options.overwrite;
    }
    scale_project(project, options.scale)?;
    project.validate()
}

fn gcd(mut a: u32, mut b: u32) -> u32 {
    while b != 0 {
        (a, b) = (b, a % b);
    }
    a
}

fn scale_project(project: &mut Project, scale: f64) -> Result<()> {
    ensure!(
        scale.is_finite() && scale > 0. && scale <= 1.,
        "--scale must be greater than 0 and at most 1"
    );
    if scale == 1. {
        return Ok(());
    }
    let w = project.video.width;
    let h = project.video.height;
    let divisor = gcd(w, h);
    // The reduced ratio has at least one odd component; an even multiplier
    // gives even dimensions without rounding the width/height independently.
    let multiplier = ((divisor as f64 * scale / 2.).round() as u32) * 2;
    let width = w / divisor * multiplier;
    let height = h / divisor * multiplier;
    ensure!(
        width >= 32 && height >= 32 && width <= w && height <= h,
        "--scale is too small for this aspect ratio; use a larger factor (minimum dimension is 32 pixels)"
    );
    let actual = width as f32 / w as f32;
    if let Some(g) = &mut project.layout.custom {
        for rect in [
            &mut g.cover,
            &mut g.title,
            &mut g.artist,
            &mut g.waveform,
            &mut g.progress,
            &mut g.time,
        ] {
            for v in rect {
                *v *= actual;
            }
        }
        g.title_size *= actual;
        g.artist_size *= actual;
        g.time_size *= actual;
    }
    project.video.width = width;
    project.video.height = height;
    eprintln!("Temporary resolution: {width}x{height} (requested --scale {scale})");
    Ok(())
}

pub fn check_destination(
    path: &Path,
    overwrite: bool,
    project: &Project,
    config: &Path,
) -> Result<()> {
    if !path.exists() {
        return Ok(());
    }
    ensure!(
        path.is_file(),
        "output is not a regular file: {}",
        path.display()
    );
    let destination = path.canonicalize()?;
    let mut inputs = vec![config];
    inputs.extend(project.fonts.latin.as_deref());
    inputs.extend(project.fonts.cjk.as_deref());
    for track in &project.tracks {
        inputs.push(&track.audio);
        inputs.extend(track.cover.as_deref());
    }
    ensure!(
        !inputs
            .iter()
            .any(|p| p.canonicalize().ok().as_ref() == Some(&destination)),
        "output would overwrite an input: {}",
        path.display()
    );
    ensure!(
        overwrite,
        "output exists: {} (use --overwrite to replace it)",
        path.display()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn project() -> Project {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        Project::load(&root.join("examples/audorchid.en.toml")).unwrap()
    }
    #[test]
    fn first_preview_and_frame_alias_have_the_same_defaults() {
        for command in ["preview", "frame"] {
            let cli = Cli::try_parse_from(["audorchid", command, "song.toml"]).unwrap();
            let Command::Frame {
                seconds, options, ..
            } = cli.command
            else {
                panic!()
            };
            assert_eq!(seconds, 0.);
            assert_eq!(options.scale, 1.);
            assert!(options.output.is_none() && !options.overwrite);
        }
        let cli = Cli::try_parse_from([
            "audorchid",
            "render",
            "song.toml",
            "-o",
            "clip.mp4",
            "--scale",
            "0.5",
            "--overwrite",
        ])
        .unwrap();
        let Command::Render { options, .. } = cli.command else {
            panic!()
        };
        assert_eq!(options.output, Some(PathBuf::from("clip.mp4")));
        assert!(options.overwrite);
        let cli = Cli::try_parse_from(["audorchid", "transition-preview", "song.toml"]).unwrap();
        let Command::TransitionPreview {
            transition,
            before,
            after,
            output,
            ..
        } = cli.command
        else {
            panic!()
        };
        assert_eq!((transition.get(), before, after), (1, 6., 6.));
        assert!(output.is_none());
        assert!(
            Cli::try_parse_from([
                "audorchid",
                "transition-preview",
                "song.toml",
                "--transition",
                "0"
            ])
            .is_err()
        );
    }
    #[test]
    fn runtime_scale_keeps_all_ratios_and_custom_geometry() {
        for (preset, w, h) in [
            (audorchid_core::Preset::Landscape, 1920, 1080),
            (audorchid_core::Preset::Portrait, 1080, 1920),
            (audorchid_core::Preset::Square, 1080, 1080),
            (audorchid_core::Preset::PortraitThreeFour, 1080, 1440),
        ] {
            let mut p = project();
            p.layout.preset = preset;
            p.video.width = w;
            p.video.height = h;
            let before = p.geometry().unwrap();
            apply_options(
                &mut p,
                &OutputOptions {
                    scale: 0.333,
                    ..Default::default()
                },
                false,
            )
            .unwrap();
            assert_eq!(
                p.video.width as u64 * h as u64,
                p.video.height as u64 * w as u64
            );
            assert!(p.video.width.is_multiple_of(2) && p.video.height.is_multiple_of(2));
            let after = p.geometry().unwrap();
            let factor = p.video.width as f32 / w as f32;
            assert!((after.progress[0] - before.progress[0] * factor).abs() < 0.01);
        }
        let mut p = project();
        let before = p.geometry().unwrap();
        p.layout.preset = audorchid_core::Preset::Custom;
        p.layout.custom = Some(before.clone());
        p.layout.progress_position = audorchid_core::ProgressPosition::Above;
        apply_options(
            &mut p,
            &OutputOptions {
                scale: 0.5,
                ..Default::default()
            },
            false,
        )
        .unwrap();
        assert_eq!(p.geometry().unwrap().cover, before.cover.map(|v| v / 2.));
        assert!(scale_project(&mut p, f64::NAN).is_err());
        assert!(scale_project(&mut p, 0.0001).is_err());
    }
    #[test]
    fn command_paths_use_terminal_and_overwrite_cannot_replace_source() {
        let mut p = project();
        let source = tempfile::NamedTempFile::new().unwrap();
        p.tracks[0].audio = source.path().to_owned();
        let input = p.tracks[0].audio.clone();
        let options = OutputOptions {
            output: Some("new-video.mp4".into()),
            overwrite: true,
            scale: 1.,
        };
        apply_options(&mut p, &options, true).unwrap();
        assert_eq!(
            p.output.path,
            std::env::current_dir().unwrap().join("new-video.mp4")
        );
        assert_eq!(p.tracks[0].audio, input);
        assert!(
            check_destination(&input, true, &p, Path::new("examples/audorchid.en.toml")).is_err()
        );
        let options = OutputOptions {
            scale: 1.,
            ..Default::default()
        };
        assert_eq!(
            preview_path(&p, &options).unwrap(),
            p.output.path.with_extension("preview.png")
        );
    }
}
