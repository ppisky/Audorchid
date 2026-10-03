mod audio_preview;
mod cli;
mod color;
mod demo;
mod demo_music;
mod mp4;
use anyhow::{Context, Result, ensure};
use audorchid_audio::{encode_aac, prepare};
use audorchid_core::{Project, VideoRateControl};
use audorchid_render::Renderer;
use clap::Parser;
use cli::{Cli, Command};
use muxide::api::{AacProfile, AudioCodec, MuxerBuilder, VideoCodec};
use openh264::{
    OpenH264API, Timestamp,
    encoder::{
        BitRate, Complexity, Encoder, EncoderConfig, FrameRate, FrameType, IntraFramePeriod,
        QpRange, RateControlMode, UsageType, VuiConfig,
    },
};
use std::time::Instant;

fn main() {
    if let Err(e) = run() {
        eprintln!("Audorchid: {e:#}");
        std::process::exit(1);
    }
}
fn run() -> Result<()> {
    let cmd = Cli::parse().command;
    match &cmd {
        Command::Demo { dir } => return demo::create(dir),
        Command::Cover {
            output,
            size,
            seed,
            overwrite,
        } => {
            let output = cli::terminal_path(output)?;
            ensure!(
                output
                    .extension()
                    .is_some_and(|s| s.eq_ignore_ascii_case("png")),
                "cover output must end in .png"
            );
            if output.exists() {
                ensure!(
                    output.is_file() && *overwrite,
                    "cover output exists (use --overwrite): {}",
                    output.display()
                );
            }
            cli::save_png(
                &audorchid_render::generate_cover(*seed, *size)?,
                &output,
                *overwrite,
            )?;
            println!("Saved {}", output.display());
            return Ok(());
        }
        _ => {}
    }
    let path = match &cmd {
        Command::Check { config }
        | Command::Inspect { config }
        | Command::Render { config, .. }
        | Command::AudioPreview { config, .. }
        | Command::TransitionPreview { config, .. }
        | Command::Frame { config, .. } => config,
        _ => unreachable!(),
    };
    let mut p = Project::load(path)?;
    match &cmd {
        Command::Render { options, .. } => cli::apply_options(&mut p, options, true)?,
        Command::Frame { options, .. } => cli::apply_options(&mut p, options, false)?,
        _ => {}
    }
    let preview = if let Command::Frame { options, .. } = &cmd {
        let output = cli::preview_path(&p, options)?;
        cli::check_destination(&output, options.overwrite, &p, path)?;
        Some(output)
    } else {
        None
    };
    let audio_destination = match &cmd {
        Command::AudioPreview {
            output, overwrite, ..
        } => Some((cli::terminal_path(output)?, *overwrite)),
        Command::TransitionPreview {
            output,
            transition,
            overwrite,
            ..
        } => Some((
            cli::transition_path(&p, output.as_deref(), transition.get())?,
            *overwrite,
        )),
        _ => None,
    };
    let audio_output = if let Some((output, overwrite)) = audio_destination {
        ensure!(
            output
                .extension()
                .is_some_and(|s| s.eq_ignore_ascii_case("wav")),
            "audio preview output must end in .wav"
        );
        cli::check_destination(&output, overwrite, &p, path)?;
        Some(output)
    } else {
        None
    };
    p.check_files()?;
    audorchid_render::validate_fonts(&p)?;
    if matches!(&cmd, Command::Render { .. }) {
        cli::check_destination(&p.output.path, p.output.overwrite, &p, path)?;
        p.check_output()?;
    }
    eprintln!("Decoding {} track(s)...", p.tracks.len());
    let audio = prepare(&p)?;
    eprintln!(
        "Loudness: {} LUFS; true peak {} dBTP; peak protection {:.2} dB (pre-AAC)",
        audio
            .loudness
            .output
            .integrated_lufs
            .map(|v| format!("{v:.2}"))
            .unwrap_or("unmeasurable".into()),
        audio
            .loudness
            .output
            .true_peak_dbtp
            .map(|v| format!("{v:.2}"))
            .unwrap_or("silence".into()),
        audio.loudness.peak_attenuation_db
    );
    let frames = audio.timeline.frames(p.video.fps_num, p.video.fps_den);
    match cmd {
        Command::Check { .. } => println!(
            "OK: {} tracks, {:.6} s, {} frames, {}x{}, gain correction {:.2} dB",
            p.tracks.len(),
            audio.timeline.seconds(),
            frames,
            p.video.width,
            p.video.height,
            audio.attenuation_db
        ),
        Command::Inspect { .. } => println!(
            "{}",
            serde_json::to_string_pretty(
                &serde_json::json!({"name":p.name,"width":p.video.width,"height":p.video.height,"fps_num":p.video.fps_num,"fps_den":p.video.fps_den,"frames":frames,"timeline":audio.timeline,"chapter_starts":audio.timeline.chapter_starts(),"layout":p.geometry()?,"spectrum_settings":p.resolved_spectrum_style()?,"visual_settings":p.visual,"attenuation_db":audio.attenuation_db,"loudness":audio.loudness,"arrangement":audio.arrangement,"transitions":audio.transitions,"video_settings":p.video,"audio_settings":p.audio,"codecs":{"video":"h264","audio":"aac-lc","container":"mp4"}})
            )?
        ),
        Command::Frame {
            seconds, options, ..
        } => {
            ensure!(
                seconds.is_finite() && seconds >= 0. && seconds < audio.timeline.seconds(),
                "--seconds must be within the timeline"
            );
            let output = preview.expect("preview destination resolved before decoding");
            let index = (seconds * p.video.fps_num as f64 / p.video.fps_den as f64).floor() as u64;
            let mut renderer = Renderer::new(&p)?;
            let image = renderer.frame(index, &p, &audio)?;
            cli::save_png(&image, &output, options.overwrite)?;
            println!("Saved {}", output.display());
        }
        Command::AudioPreview {
            seconds,
            duration,
            overwrite,
            ..
        } => {
            let output = audio_output.expect("audio preview destination resolved before decoding");
            audio_preview::save(&audio, seconds, duration, &output, overwrite)?;
            println!("Saved {}", output.display());
        }
        Command::TransitionPreview {
            transition,
            before,
            after,
            overwrite,
            ..
        } => {
            let transition = transition.get();
            let range =
                audio_preview::transition_range(&audio.timeline, transition, before, after)?;
            let outgoing = audio.arrangement.order[transition - 1];
            let incoming = audio.arrangement.order[transition];
            eprintln!(
                "Transition {transition}: {} -> {}; {:.3} s overlap; excerpt {:.3}..{:.3} s",
                p.tracks[outgoing].title,
                p.tracks[incoming].title,
                audio.timeline.incoming_overlap(transition) as f64 / p.audio.sample_rate as f64,
                range.start as f64 / p.audio.sample_rate as f64,
                range.end as f64 / p.audio.sample_rate as f64,
            );
            let output = audio_output.expect("audio preview destination resolved before decoding");
            audio_preview::save_range(&audio, range, &output, overwrite)?;
            println!("Saved {}", output.display());
        }
        Command::Render { .. } => {
            let started = Instant::now();
            eprintln!("Encoding AAC-LC...");
            let aac = encode_aac(&audio.pcm, p.audio.sample_rate, p.audio.bitrate_kbps)?;
            eprintln!(
                "AAC: priming {} samples, padding {} samples; {} frames",
                aac.delay_samples,
                aac.padding_samples,
                aac.packets.len()
            );
            let parent = p.output.path.parent().context("output has no parent")?;
            std::fs::create_dir_all(parent)?;
            let mut temp = tempfile::Builder::new()
                .prefix(".audorchid-")
                .suffix(".mp4.part")
                .tempfile_in(parent)?;
            {
                let mut mux = MuxerBuilder::new(temp.as_file_mut())
                    .video(
                        VideoCodec::H264,
                        p.video.width,
                        p.video.height,
                        p.video.fps_num as f64 / p.video.fps_den as f64,
                    )
                    .audio(AudioCodec::Aac(AacProfile::Lc), p.audio.sample_rate, 2)
                    .with_fast_start(false)
                    .build()?;
                let config = EncoderConfig::new()
                    // UI text and thin visualizer bars are screen content.
                    // Select the encoder's screen-content mode and disable
                    // camera-oriented background classification for this scene.
                    .usage_type(UsageType::ScreenContentRealTime)
                    .background_detection(false)
                    .bitrate(BitRate::from_bps(p.video.bitrate_kbps * 1000))
                    .rate_control_mode(match p.video.rate_control {
                        VideoRateControl::Quality => RateControlMode::Quality,
                        VideoRateControl::Bitrate => RateControlMode::Bitrate,
                    })
                    .qp(match p.video.rate_control {
                        VideoRateControl::Quality => {
                            QpRange::new(p.video.quality_qp, p.video.quality_qp)
                        }
                        VideoRateControl::Bitrate => QpRange::new(0, 51),
                    })
                    .complexity(Complexity::High)
                    .adaptive_quantization(false)
                    .skip_frames(false)
                    .max_frame_rate(FrameRate::from_hz(
                        p.video.fps_num as f32 / p.video.fps_den as f32,
                    ))
                    .intra_frame_period(IntraFramePeriod::from_num_frames(
                        p.video.keyframe_interval,
                    ))
                    .vui(VuiConfig::bt709());
                let mut encoder = Encoder::with_api_config(OpenH264API::from_source(), config)?;
                let mut renderer = Renderer::new(&p)?;
                let color = color::ColorConverter::default();
                let mut last_progress = Instant::now();
                let mut encoded_bytes = 0u64;
                let mut audio_index = 0usize;
                for frame in 0..frames {
                    let rgba = renderer
                        .frame(frame, &p, &audio)
                        .with_context(|| format!("render frame {frame}"))?;
                    let yuv = color.convert(
                        rgba.as_raw(),
                        p.video.width as usize,
                        p.video.height as usize,
                    )?;
                    let millis = (frame as u128 * p.video.fps_den as u128 * 1000
                        / p.video.fps_num as u128) as u64;
                    let encoded = encoder.encode_at(&yuv, Timestamp::from_millis(millis))?;
                    let keyframe = matches!(encoded.frame_type(), FrameType::IDR | FrameType::I);
                    let packet = encoded.to_vec();
                    ensure!(!packet.is_empty(), "encoder skipped frame {frame}");
                    encoded_bytes += packet.len() as u64;
                    ensure!(
                        encoded_bytes < 1_500_000_000,
                        "current muxide backend buffers packets; 1.5 GB encoded video safety limit exceeded"
                    );
                    let pts = frame as f64 * p.video.fps_den as f64 / p.video.fps_num as f64;
                    mux.write_video(pts, &packet, keyframe)?;
                    while audio_index < aac.packets.len()
                        && audio_index as f64 * aac.frame_samples as f64
                            / p.audio.sample_rate as f64
                            <= pts
                    {
                        mux.write_audio(
                            audio_index as f64 * aac.frame_samples as f64
                                / p.audio.sample_rate as f64,
                            &aac.packets[audio_index],
                        )?;
                        audio_index += 1;
                    }
                    if last_progress.elapsed().as_secs_f32() > 2. || frame + 1 == frames {
                        eprintln!(
                            "{}/{} frames ({:.1}%), {:.1} fps",
                            frame + 1,
                            frames,
                            (frame + 1) as f64 / frames as f64 * 100.,
                            (frame + 1) as f64 / started.elapsed().as_secs_f64()
                        );
                        last_progress = Instant::now();
                    }
                }
                for (i, packet) in aac.packets.iter().enumerate().skip(audio_index) {
                    mux.write_audio(
                        i as f64 * aac.frame_samples as f64 / p.audio.sample_rate as f64,
                        packet,
                    )?;
                }
                mux.finish()?;
            }
            let video_duration =
                (frames as u128 * p.video.fps_den as u128 * p.audio.sample_rate as u128)
                    .div_ceil(p.video.fps_num as u128) as u64;
            mp4::apply_audio_edit(
                temp.as_file_mut(),
                p.audio.sample_rate,
                aac.delay_samples,
                aac.valid_samples,
                aac.packets.len() as u64,
                aac.frame_samples,
                video_duration,
            )?;
            temp.as_file().sync_all()?;
            if p.output.overwrite {
                temp.persist(&p.output.path)?;
            } else {
                temp.persist_noclobber(&p.output.path)?;
            }
            println!(
                "Saved {} ({:.2} s elapsed)",
                p.output.path.display(),
                started.elapsed().as_secs_f64()
            );
        }
        Command::Demo { .. } | Command::Cover { .. } => unreachable!(),
    }
    Ok(())
}
