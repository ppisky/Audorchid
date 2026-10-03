use crate::fonts::{FontNames, is_cjk};
use anyhow::Result;
use audorchid_audio::{Peak, SpectrumFrame};
use audorchid_core::{Geometry, Project};
use gpui::{prelude::*, *};
use image::RgbaImage;
use std::{cell::RefCell, path::Path as FilePath, rc::Rc, sync::Arc};

#[derive(Clone, Default)]
pub(crate) struct FrameState {
    pub seconds: f64,
    pub active: Option<usize>,
    pub crossfade: Option<(usize, usize, f32)>,
    pub progress: f32,
    pub elapsed: f64,
    pub duration: f64,
    pub energy: f32,
    pub waveform: Vec<Peak>,
    pub spectrum: SpectrumFrame,
    pub scope: Vec<[f32; 2]>,
    pub overlaps: Vec<[f32; 2]>,
    pub source_order: Vec<usize>,
}
pub(crate) struct Artwork {
    cover: Arc<RenderImage>,
    background: Arc<RenderImage>,
}
fn render_image(mut image: RgbaImage) -> Arc<RenderImage> {
    // GPUI's RenderImage consumes BGRA even though image::RgbaImage stores bytes.
    for pixel in image.pixels_mut() {
        pixel.0.swap(0, 2);
    }
    Arc::new(RenderImage::new(vec![image::Frame::new(image)]))
}
impl Artwork {
    pub fn new(path: Option<&FilePath>, seed: u64, p: &Project) -> Result<Self> {
        let image = match path {
            // Downloads can have a .jpg name while containing PNG data.
            Some(path) => image::ImageReader::open(path)?
                .with_guessed_format()?
                .decode()?
                .to_rgba8(),
            None => crate::generate_cover(seed, 640)?,
        };
        let r = p.geometry()?.cover;
        let cover = image::imageops::resize(
            &image::imageops::crop_imm(
                &image,
                (image.width() - image.width().min(image.height())) / 2,
                (image.height() - image.width().min(image.height())) / 2,
                image.width().min(image.height()),
                image.width().min(image.height()),
            )
            .to_image(),
            r[2].ceil() as u32,
            r[3].ceil() as u32,
            image::imageops::FilterType::Lanczos3,
        );
        let mut background = image::imageops::blur(
            &image::imageops::resize(&image, 96, 96, image::imageops::FilterType::Triangle),
            p.visual.artwork_style.background_blur,
        );
        for (x, y, pixel) in background.enumerate_pixels_mut() {
            let shade =
                0.42 + 0.20 * (1. - ((x as f32 / 96. - 0.35).hypot(y as f32 / 96. - 0.3)).min(1.));
            for c in &mut pixel.0[..3] {
                *c = (*c as f32 * shade + 6.) as u8;
            }
            pixel.0[3] = 255;
        }
        Ok(Self {
            cover: render_image(cover),
            background: render_image(background),
        })
    }
}
pub(crate) struct MusicScene {
    pub project: Project,
    pub geometry: Geometry,
    pub fonts: FontNames,
    pub artworks: Arc<Vec<Artwork>>,
    pub frame: FrameState,
    pub error: Rc<RefCell<Option<String>>>,
}
impl Render for MusicScene {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let p = self.project.clone();
        let mut g = self.geometry.clone();
        let fonts = self.fonts.clone();
        let artworks = self.artworks.clone();
        let f = self.frame.clone();
        let error = self.error.clone();
        canvas(
            |_, _, _| (),
            move |_, _, window, cx| {
                let dpi = window.scale_factor();
                for r in [
                    &mut g.cover,
                    &mut g.title,
                    &mut g.artist,
                    &mut g.waveform,
                    &mut g.progress,
                    &mut g.time,
                ] {
                    for v in r {
                        *v /= dpi;
                    }
                }
                g.title_size /= dpi;
                g.artist_size /= dpi;
                g.time_size /= dpi;
                if let Err(e) = paint(&p, &g, &fonts, &artworks, &f, window, cx) {
                    *error.borrow_mut() = Some(e.to_string());
                }
            },
        )
        .size_full()
    }
}
fn bounds(r: [f32; 4]) -> Bounds<Pixels> {
    Bounds {
        origin: point(px(r[0]), px(r[1])),
        size: size(px(r[2]), px(r[3])),
    }
}
pub(crate) fn rectangle(w: &mut Window, r: [f32; 4], color: Hsla, radius: f32) {
    w.paint_quad(fill(bounds(r), color).corner_radii(px(radius)));
}
fn image(w: &mut Window, r: [f32; 4], image: Arc<RenderImage>, radius: f32) -> Result<()> {
    w.paint_image(bounds(r), Corners::all(px(radius)), image, 0, false)
}
pub(crate) fn rand(seed: u64, id: u64) -> f32 {
    let mut x = seed.wrapping_add(id.wrapping_mul(0x9e3779b97f4a7c15));
    x = (x ^ (x >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94d049bb133111eb);
    ((x ^ (x >> 31)) >> 40) as f32 / 16_777_216.
}
fn paint(
    p: &Project,
    g: &Geometry,
    fonts: &FontNames,
    art: &[Artwork],
    f: &FrameState,
    w: &mut Window,
    cx: &mut App,
) -> Result<()> {
    let width = p.video.width as f32 / w.scale_factor();
    let height = p.video.height as f32 / w.scale_factor();
    let scale = width.min(height) / 1080.;
    let all = [0., 0., width, height];
    let active = f.active.unwrap_or(0);
    let (first, second, alpha) = f.crossfade.unwrap_or((active, active, 0.));
    let source = |index: usize| f.source_order.get(index).copied().unwrap_or(index);
    let (first, second) = (source(first), source(second));
    image(w, all, art[first].background.clone(), 0.)?;
    if first != second {
        w.with_subtree_effect_chain(bounds(all), &[], alpha, |w| {
            image(w, all, art[second].background.clone(), 0.)
        })?;
    }
    crate::visuals::triangles(w, p, f, width, height, scale)?;
    if f.active.is_none() {
        return Ok(());
    }
    let label = [
        g.cover[0],
        g.cover[1] - 55. * scale,
        width - g.cover[0] * 2.,
        28. * scale,
    ];
    text(
        w,
        cx,
        fonts,
        "A U D O R C H I D",
        label,
        18. * scale,
        hsla(0., 0., 1., 0.58),
        false,
        false,
    )?;
    let radius = p.visual.artwork_style.corner_radius * scale;
    let border = p.visual.artwork_style.border_width * scale;
    rectangle(
        w,
        [
            g.cover[0] - border,
            g.cover[1] - border,
            g.cover[2] + 2. * border,
            g.cover[3] + 2. * border,
        ],
        hsla(0., 0., 1., 0.12),
        if radius > 0. {
            radius + border * 2. / 3.
        } else {
            0.
        },
    );
    image(w, g.cover, art[first].cover.clone(), radius)?;
    if first != second {
        w.with_subtree_effect_chain(bounds(g.cover), &[], alpha, |w| {
            image(w, g.cover, art[second].cover.clone(), radius)
        })?;
    }
    let track = &p.tracks[source(active)];
    // Title, chapter counter and markers share the same exact midpoint.
    // Keep full opacity at handoff instead of fading the title to nothing.
    let title_alpha = 1.;
    text(
        w,
        cx,
        fonts,
        &track.title,
        g.title,
        g.title_size,
        hsla(0., 0., 0.97, title_alpha),
        true,
        false,
    )?;
    text(
        w,
        cx,
        fonts,
        &track.artist,
        g.artist,
        g.artist_size,
        hsla(0., 0., 0.82, title_alpha * 0.85),
        false,
        false,
    )?;
    crate::visuals::audio(w, p, g, f, scale)?;
    let style = &p.visual.progress_style;
    let radius = (style.corner_radius * scale).min(g.progress[3] / 2.);
    rectangle(
        w,
        g.progress,
        crate::visuals::color(&style.track_color, style.track_opacity)?,
        radius,
    );
    if f.progress > 0. {
        let mut r = g.progress;
        r[2] *= f.progress;
        rectangle(w, r, crate::visuals::color(&style.color, 1.)?, radius);
    }
    if style.show_overlap {
        for &[start, end] in &f.overlaps {
            let r = [
                g.progress[0] + start * g.progress[2],
                g.progress[1],
                (end - start) * g.progress[2],
                g.progress[3],
            ];
            rectangle(
                w,
                r,
                crate::visuals::color(&style.overlap_color, style.overlap_opacity)?,
                radius,
            );
        }
    }
    let time = format!("{} / {}", format_time(f.elapsed), format_time(f.duration));
    let mut time_bounds = g.time;
    time_bounds[2] = (time_bounds[2] - g.time_size * 6.).max(g.time_size);
    text(
        w,
        cx,
        fonts,
        &time,
        time_bounds,
        g.time_size,
        hsla(0., 0., 1., 0.62),
        false,
        false,
    )?;
    if !style.show_counter {
        return Ok(());
    }
    let count = format!("{:02} / {:02}", active + 1, p.tracks.len());
    let mut r = g.time;
    r[0] += r[2] - g.time_size * 5. - style.counter_right_inset * scale;
    r[2] = g.time_size * 5.;
    text(
        w,
        cx,
        fonts,
        &count,
        r,
        g.time_size,
        hsla(0., 0., 1., 0.5),
        false,
        true,
    )?;
    Ok(())
}
fn format_time(t: f64) -> String {
    let s = t.max(0.) as u64;
    format!("{:02}:{:02}", s / 60, s % 60)
}
fn runs(text: &str, fonts: &FontNames, color: Hsla) -> Vec<TextRun> {
    let mut runs: Vec<TextRun> = Vec::new();
    for c in text.chars() {
        let family = if is_cjk(c) {
            fonts.cjk.as_ref().unwrap_or(&fonts.latin)
        } else {
            &fonts.latin
        };
        let mut f = font(family.clone());
        f.features = FontFeatures(Arc::new(vec![
            ("tnum".into(), 1),
            ("lnum".into(), 1),
            ("pnum".into(), 0),
        ]));
        if let Some(last) = runs.last_mut().filter(|r| r.font == f) {
            last.len += c.len_utf8();
        } else {
            runs.push(TextRun {
                len: c.len_utf8(),
                font: f,
                color,
                ..Default::default()
            });
        }
    }
    runs
}
fn fit_ink_y(ink: [f32; 4], baseline: f32, top: f32, bottom: f32, dpi: f32) -> Result<f32> {
    // At small preview sizes a mandatory one-pixel padding on BOTH sides
    // can consume more space than the text itself. Keep all ink, then use
    // whatever padding fits, up to one output pixel per side.
    let margin = ((bottom - top - (ink[3] - ink[1])) / 2.).clamp(0., 1. / dpi);
    let min_y = top + margin - baseline - ink[1];
    let max_y = bottom - margin - baseline - ink[3];
    anyhow::ensure!(
        ink[3] - ink[1] <= bottom - top + 0.0001 / dpi,
        "text box too short for font ink: increase height or reduce font size"
    );
    // With no spare padding, the two bounds are mathematically identical but
    // floating-point association may reverse them by a fraction of a pixel.
    Ok(top.max(min_y).min(max_y))
}
#[expect(
    clippy::too_many_arguments,
    reason = "Keep GPUI context and the explicit per-call text style together at the canvas boundary"
)]
fn text(
    w: &mut Window,
    cx: &mut App,
    fonts: &FontNames,
    value: &str,
    r: [f32; 4],
    font_size: f32,
    color: Hsla,
    multiline: bool,
    right_align: bool,
) -> Result<()> {
    let value = value.replace(['\n', '\r'], " ");
    let line_height = font_size * 1.2;
    let max_lines = if multiline {
        (r[3] / line_height).floor().max(1.) as usize
    } else {
        1
    };
    // Shape with actual font metrics. Wrap on scalar boundaries; never split UTF-8.
    let chars: Vec<_> = value.chars().collect();
    let mut offset = 0;
    for row in 0..max_lines {
        if offset >= chars.len() {
            break;
        }
        let mut end = offset;
        let mut current = String::new();
        while end < chars.len() {
            current.push(chars[end]);
            let shaped = w.text_system().shape_line(
                current.clone().into(),
                px(font_size),
                &runs(&current, fonts, color),
                None,
            );
            if shaped.width() > px(r[2]) {
                current.pop();
                break;
            }
            end += 1;
        }
        if end == offset {
            break;
        }
        if row + 1 == max_lines && end < chars.len() {
            loop {
                let candidate = format!("{current}...");
                let shaped = w.text_system().shape_line(
                    candidate.clone().into(),
                    px(font_size),
                    &runs(&candidate, fonts, color),
                    None,
                );
                if shaped.width() <= px(r[2]) || current.is_empty() {
                    current = candidate;
                    break;
                }
                current.pop();
            }
        }
        let shaped = w.text_system().shape_line(
            current.clone().into(),
            px(font_size),
            &runs(&current, fonts, color),
            None,
        );
        let ink = fonts.ink_bounds(&shaped, w.scale_factor())?;
        let baseline = (line_height + f32::from(shaped.ascent) - f32::from(shaped.descent)) / 2.;
        let row_top = r[1] + row as f32 * line_height;
        let row_bottom = if row + 1 == max_lines {
            r[1] + r[3]
        } else {
            (row_top + line_height).min(r[1] + r[3])
        };
        // Fit real raster ink inside the row. Mixed CJK/Latin metrics can put
        // Latin descenders below the nominal line box even at a 1.2 line height.
        let y = fit_ink_y(ink, baseline, row_top, row_bottom, w.scale_factor())?;
        let x = if right_align {
            r[0] + r[2] - ink[2]
        } else {
            r[0] - ink[0].min(0.)
        };
        w.with_content_mask(Some(ContentMask { bounds: bounds(r) }), |w| {
            shaped.paint(
                point(px(x), px(y)),
                px(line_height),
                TextAlign::Left,
                None,
                w,
                cx,
            )
        })?;
        offset = end;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::prelude::v1::test;
    #[test]
    fn mixed_font_raster_descenders_fit_all_preset_title_sizes() {
        let root = FilePath::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let p = Project::load(&root.join("examples/audorchid.en.toml")).unwrap();
        let loaded = crate::fonts::load(&p).unwrap();
        let platform = Arc::new(gpui_wgpu::CosmicTextSystem::new_without_system_fonts(
            &loaded.latin_family,
        ));
        platform
            .add_fonts(vec![
                std::borrow::Cow::Owned(loaded.latin_bytes.clone()),
                std::borrow::Cow::Owned(loaded.cjk_bytes.clone().unwrap()),
            ])
            .unwrap();
        let names = loaded.names(platform.clone());
        let mut reproduced_old_clip = false;
        for dpi in [1., 2.] {
            for (size, height) in [
                (70., 84.),
                (64., 77.),
                (48., 58.),
                (54., 65.),
                (70. / 3., 84. / 3.),
                (18. / 3., 28. / 3.),
                (22. / 3., 30. / 3.),
            ] {
                for value in [
                    "晨光 / Morning light",
                    "gy pq j",
                    "晨光 gy pq j / 中文",
                    "Ágypqj",
                ] {
                    let font_runs: Vec<_> = runs(value, &names, white())
                        .into_iter()
                        .map(|run| FontRun {
                            len: run.len,
                            font_id: platform.font_id(&run.font).unwrap(),
                        })
                        .collect();
                    let line = platform.layout_line(value, px(size / dpi), &font_runs);
                    let ink = names.ink_bounds(&line, dpi).unwrap();
                    let line_height = size / dpi * 1.2;
                    let baseline =
                        (line_height + f32::from(line.ascent) - f32::from(line.descent)) / 2.;
                    let bottom = height / dpi;
                    reproduced_old_clip |= baseline + ink[3] > bottom;
                    let y = fit_ink_y(ink, baseline, 0., bottom, dpi).unwrap_or_else(|e| panic!("{e}; value={value}, size={size}, height={height}, dpi={dpi}, ink={ink:?}"));
                    assert!(y + baseline + ink[1] >= -0.001);
                    assert!(y + baseline + ink[3] <= bottom + 0.001);
                }
            }
        }
        assert!(
            reproduced_old_clip,
            "fixture must expose the original descender clipping"
        );
        assert!(fit_ink_y([0., -30., 30., 10.], 30., 0., 20., 1.).is_err());
    }
}
