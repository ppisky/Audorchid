//! Deterministic GPUI scenes rendered by wgpu without a window.
mod artwork;
mod fonts;
mod gpu;
mod scene;
mod visuals;
use anyhow::{Context as _, Result, bail};
pub use artwork::generate_cover;
use audorchid_audio::{PreparedAudio, Spectrum, audible, scope};
use audorchid_core::Project;
pub use fonts::validate_fonts;
use gpu::GpuRenderer;
use gpui::*;
use gpui_wgpu::CosmicTextSystem;
use scene::{Artwork, FrameState, MusicScene};
use std::{borrow::Cow, cell::RefCell, rc::Rc, sync::Arc};

struct Offscreen {
    renderer: GpuRenderer,
}
impl PlatformHeadlessRenderer for Offscreen {
    fn sprite_atlas(&self) -> Arc<dyn PlatformAtlas> {
        self.renderer.sprite_atlas()
    }
    fn render_scene(&mut self, _scene: &Scene, _size: Size<DevicePixels>) -> Result<()> {
        // Export explicitly captures the completed scene, propagating any error.
        Ok(())
    }
    fn render_scene_to_image(
        &mut self,
        scene: &Scene,
        size: Size<DevicePixels>,
    ) -> Result<image::RgbaImage> {
        self.renderer.resize(size);
        for attempt in 0..3 {
            match self.renderer.render_rgba(scene) {
                Ok(rgba) => {
                    return image::RgbaImage::from_raw(
                        size.width.0 as u32,
                        size.height.0 as u32,
                        rgba,
                    )
                    .context("invalid RGBA readback size");
                }
                Err(e) if attempt < 2 && e.to_string().contains("larger instance buffer") => {
                    continue;
                }
                Err(e) => return Err(e),
            }
        }
        unreachable!()
    }
}

pub struct Renderer {
    window: WindowHandle<MusicScene>,
    entity: Entity<MusicScene>,
    paint_error: Rc<RefCell<Option<String>>>,
    spectrum: Option<Spectrum>,
    // The external entity handle must drop before HeadlessAppContext performs
    // its shutdown and leak audit. Rust drops fields in declaration order.
    cx: HeadlessAppContext,
}
impl Renderer {
    pub fn new(project: &Project) -> Result<Self> {
        let fonts = fonts::load(project)?;
        let text = Arc::new(CosmicTextSystem::new_without_system_fonts(
            &fonts.latin_family,
        ));
        let mut bytes = vec![Cow::Owned(fonts.latin_bytes.clone())];
        if let Some(data) = &fonts.cjk_bytes {
            bytes.push(Cow::Owned(data.clone()));
        }
        text.add_fonts(bytes)?;
        text.font_id(&font(fonts.latin_family.clone()))
            .context("cannot resolve Latin font family")?;
        if let Some(name) = &fonts.cjk_family {
            text.font_id(&font(name.clone()))
                .context("cannot resolve CJK font family")?;
        }
        let renderer = GpuRenderer::new(size(
            DevicePixels(project.video.width as i32),
            DevicePixels(project.video.height as i32),
        ))?;
        let slot = RefCell::new(Some(renderer));
        let mut cx = HeadlessAppContext::with_platform(text.clone(), Arc::new(()), move || {
            slot.borrow_mut().take().map(|renderer| {
                Box::new(Offscreen { renderer }) as Box<dyn PlatformHeadlessRenderer>
            })
        });
        let paint_error = Rc::new(RefCell::new(None));
        let artworks = project
            .tracks
            .iter()
            .enumerate()
            .map(|(i, t)| {
                Artwork::new(
                    t.cover.as_deref(),
                    t.cover_seed.unwrap_or(i as u64),
                    project,
                )
            })
            .collect::<Result<Vec<_>>>()?;
        let scene = MusicScene {
            project: project.clone(),
            geometry: project.geometry()?,
            fonts: fonts.names(text),
            artworks: Arc::new(artworks),
            frame: FrameState::default(),
            error: paint_error.clone(),
        };
        let entity = cx.new(|_| scene);
        let root = entity.clone();
        let window = cx.open_window(
            size(
                px(project.video.width as f32),
                px(project.video.height as f32),
            ),
            move |_, _| root,
        )?;
        cx.update_window(window.into(), |_, window, cx| {
            let scale = window.scale_factor();
            window.resize(size(
                px(project.video.width as f32 / scale),
                px(project.video.height as f32 / scale),
            ));
            window.bounds_changed(cx);
        })?;
        Ok(Self {
            cx,
            window,
            entity,
            paint_error,
            spectrum: None,
        })
    }
    pub fn frame(
        &mut self,
        index: u64,
        p: &Project,
        audio: &PreparedAudio,
    ) -> Result<image::RgbaImage> {
        let sample = audio
            .timeline
            .sample_at_frame(index, p.video.fps_num, p.video.fps_den);
        let active = audio.timeline.active_track(sample);
        let has_audio = active.is_some() && audible(&audio.pcm, sample, p.audio.sample_rate);
        let mut crossfade = None;
        for (i, next) in audio.timeline.tracks.iter().enumerate().skip(1) {
            let overlap = audio.timeline.incoming_overlap(i);
            if overlap > 0 && sample >= next.start && sample < next.start + overlap {
                crossfade = Some((i - 1, i, (sample - next.start) as f32 / overlap as f32));
                break;
            }
        }
        let progress = audio
            .timeline
            .progress(sample, p.visual.progress_style.scope);
        let spectrum = if has_audio && p.visual.spectrum {
            let style = p.resolved_spectrum_style()?;
            self.spectrum
                .get_or_insert_with(|| Spectrum::new(&audio.pcm, p.audio.sample_rate, &style))
                .at(sample)
        } else {
            Default::default()
        };
        let state = FrameState {
            seconds: index as f64 * p.video.fps_den as f64 / p.video.fps_num as f64,
            active,
            crossfade,
            progress: progress.fraction,
            elapsed: progress.elapsed,
            duration: progress.duration,
            overlaps: progress.overlaps,
            source_order: audio.arrangement.order.clone(),
            energy: audio.analysis.energy(sample),
            waveform: if has_audio && p.visual.waveform {
                audio.analysis.waveform(sample, 180)
            } else {
                Vec::new()
            },
            scope: if has_audio && p.visual.waveform {
                scope(
                    &audio.pcm,
                    sample,
                    p.audio.sample_rate,
                    p.visual.waveform_style.window_ms,
                    1024,
                )
            } else {
                Vec::new()
            },
            spectrum,
        };
        self.paint_error.borrow_mut().take();
        self.cx.update_entity(&self.entity, |view, cx| {
            view.frame = state;
            cx.notify();
        });
        let arena = self.cx.update_window(self.window.into(), |_, window, cx| {
            window.refresh();
            window.draw(cx)
        })?;
        arena.clear();
        if let Some(error) = self.paint_error.borrow_mut().take() {
            bail!("GPUI paint: {error}");
        }
        let image = self.cx.capture_screenshot(self.window.into())?;
        anyhow::ensure!(
            image.dimensions() == (p.video.width, p.video.height),
            "offscreen dimensions do not match video configuration"
        );
        Ok(image)
    }
}
