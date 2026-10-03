//! Run explicitly after `audorchid demo`; requires a working wgpu device.
use audorchid_audio::prepare;
use audorchid_core::{Project, TriangleMode, TrianglePreset};
use audorchid_render::Renderer;
use std::path::Path;

#[test]
#[ignore = "requires GPU and generated examples/media; run with --ignored"]
fn automatic_order_matches_manually_reordered_audio_and_images() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut automatic = Project::load(&root.join("examples/audorchid.en.toml")).unwrap();
    automatic.audio.arrangement.mode = audorchid_core::ArrangementMode::Automatic;
    let prepared = prepare(&automatic).unwrap();
    assert_ne!(
        prepared.arrangement.order,
        vec![0, 1],
        "fixture must actually reorder tracks"
    );
    let mut manual = automatic.clone();
    manual.audio.arrangement.mode = audorchid_core::ArrangementMode::Configuration;
    manual.tracks = prepared
        .arrangement
        .order
        .iter()
        .map(|&i| automatic.tracks[i].clone())
        .collect();
    let expected = prepare(&manual).unwrap();
    assert_eq!(prepared.pcm, expected.pcm);
    let mut auto_renderer = Renderer::new(&automatic).unwrap();
    let mut manual_renderer = Renderer::new(&manual).unwrap();
    for frame in [0, 449, 450, 451, 600] {
        assert!(
            auto_renderer.frame(frame, &automatic, &prepared).unwrap()
                == manual_renderer.frame(frame, &manual, &expected).unwrap(),
            "reordered title/cover/progress differ at frame {frame}"
        );
    }
}

#[test]
#[ignore = "requires GPU and generated examples/media; run with --ignored"]
fn sequential_frames_match_a_cold_seek_at_track_handoff() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let project = Project::load(&root.join("examples/audorchid.en.toml")).unwrap();
    let audio = prepare(&project).unwrap();
    let chapter = audio.timeline.chapter_starts()[1];
    let handoff = (chapter as u128 * project.video.fps_num as u128)
        .div_ceil(project.audio.sample_rate as u128 * project.video.fps_den as u128)
        as u64;
    let cold = Renderer::new(&project)
        .unwrap()
        .frame(handoff, &project, &audio)
        .unwrap();
    let mut renderer = Renderer::new(&project).unwrap();
    for index in (0..=handoff + 2).chain([handoff + 30, handoff]) {
        let frame = renderer.frame(index, &project, &audio).unwrap();
        if index == handoff {
            if frame != cold {
                std::fs::create_dir_all(root.join("output")).unwrap();
                frame.save(root.join("output/sequence-actual.png")).unwrap();
                cold.save(root.join("output/sequence-expected.png"))
                    .unwrap();
            }
            assert!(frame == cold, "sequential frame differs from cold seek");
        }
    }
    // Triangles are now opt-in; exercise both presets while seeking as well.
    for preset in [TrianglePreset::Layered, TrianglePreset::Border] {
        let mut project = project.clone();
        project.visual.triangles = 48;
        project.visual.triangle_style.preset = preset;
        project.visual.triangle_style.mode = TriangleMode::Auto;
        let cold = Renderer::new(&project)
            .unwrap()
            .frame(handoff, &project, &audio)
            .unwrap();
        let mut renderer = Renderer::new(&project).unwrap();
        renderer.frame(handoff + 30, &project, &audio).unwrap();
        renderer.frame(0, &project, &audio).unwrap();
        assert!(renderer.frame(handoff, &project, &audio).unwrap() == cold);
    }
}
