//! Independently implemented geometry and audio displays; no osu/VLC code.
use crate::scene::{FrameState, rand, rectangle};
use anyhow::Result;
use audorchid_core::{
    Geometry, Project, SpectrumPalette, TriangleMode, TrianglePreset, WaveformMode, rgb_hex,
};
use gpui::*;

pub(crate) fn color(hex: &str, alpha: f32) -> Result<Hsla> {
    let mut value: Hsla = rgb(rgb_hex(hex)?).into();
    value.a = alpha.clamp(0., 1.);
    Ok(value)
}
pub(crate) fn triangles(
    w: &mut Window,
    p: &Project,
    f: &FrameState,
    width: f32,
    height: f32,
    scale: f32,
) -> Result<()> {
    let style = &p.visual.triangle_style;
    let mode = match style.mode {
        TriangleMode::Auto if style.preset == TrianglePreset::Border => TriangleMode::Outline,
        TriangleMode::Auto => TriangleMode::Filled,
        mode => mode,
    };
    let dark = rgb_hex(&style.color)?;
    let light = rgb_hex(&style.color_light)?;
    for i in 0..p.visual.triangles as u64 {
        // A bounded bell-shaped distribution favours medium sizes/speeds.
        // Every attribute comes from an absolute-time seed, not frame history.
        let u = (1. - rand(p.visual.seed, i * 7)).max(1e-6);
        let v = rand(p.visual.seed, i * 7 + 1);
        let variation = (0.5 + 0.18 * (-2. * u.ln()).sqrt() * (v * std::f32::consts::TAU).cos())
            .clamp(0.05, 0.95);
        let size = match style.preset {
            TrianglePreset::Layered => {
                style.min_size + (style.max_size - style.min_size) * variation
            }
            TrianglePreset::Border => style.size,
        } * scale;
        let h = size * 0.8660254;
        let speed = style.speed * (0.5 + variation) * scale;
        let distance = rand(p.visual.seed, i * 7 + 2) as f64 * (height + 2. * h) as f64
            - f.seconds * speed as f64;
        let cycle = distance.div_euclid((height + 2. * h) as f64) as i64;
        let x = rand(p.visual.seed.wrapping_add(cycle as u64), i * 7 + 3) * width;
        let y = distance.rem_euclid((height + 2. * h) as f64) as f32 - h;
        let shade = match style.preset {
            TrianglePreset::Layered => rand(p.visual.seed, i * 7 + 4),
            TrianglePreset::Border => ((y + h / 2.) / height).clamp(0., 1.),
        };
        let mut tint = 0u32;
        for shift in [0, 8, 16] {
            let a = ((dark >> shift) & 255) as f32;
            let b = ((light >> shift) & 255) as f32;
            tint |= ((a + (b - a) * shade).round() as u32) << shift;
        }
        let mut ink: Hsla = rgb(tint).into();
        ink.a = (style.opacity * (1. + f.energy * style.music_reactivity)).clamp(0., 1.);
        for filled in [true, false] {
            if (filled && mode == TriangleMode::Outline)
                || (!filled && mode == TriangleMode::Filled)
            {
                continue;
            }
            if !filled && style.preset == TrianglePreset::Border {
                if style.thickness == 0. {
                    continue;
                }
                // Three non-overlapping strips form an inward border. A value
                // of 1 collapses its hole to the centroid (a solid triangle).
                let outer = [[x, y], [x + size / 2., y + h], [x - size / 2., y + h]];
                let center = [x, y + h * 2. / 3.];
                let inner = outer.map(|p| {
                    [
                        center[0] + (p[0] - center[0]) * (1. - style.thickness),
                        center[1] + (p[1] - center[1]) * (1. - style.thickness),
                    ]
                });
                let mut path = PathBuilder::fill();
                for edge in 0..3 {
                    let next = (edge + 1) % 3;
                    path.move_to(point(px(outer[edge][0]), px(outer[edge][1])));
                    for p in [outer[next], inner[next], inner[edge]] {
                        path.line_to(point(px(p[0]), px(p[1])));
                    }
                    path.close();
                }
                w.paint_path(path.build()?, ink);
                continue;
            }
            let mut path = if filled {
                PathBuilder::fill()
            } else {
                PathBuilder::stroke(px(style.stroke_width * scale))
            };
            path.move_to(point(px(x), px(y)));
            path.line_to(point(px(x + size / 2.), px(y + h)));
            path.line_to(point(px(x - size / 2.), px(y + h)));
            path.close();
            w.paint_path(path.build()?, ink);
        }
    }
    Ok(())
}
pub(crate) fn audio(
    w: &mut Window,
    p: &Project,
    g: &Geometry,
    f: &FrameState,
    scale: f32,
) -> Result<()> {
    let mut wave = g.waveform;
    let mut spec = g.waveform;
    if p.visual.waveform && p.visual.spectrum {
        // Two enabled modes get separate space; they do not obscure each other.
        wave[3] *= 0.4;
        spec[1] += spec[3] * 0.48;
        spec[3] *= 0.52;
    }
    if p.visual.waveform {
        let style = &p.visual.waveform_style;
        let ink = color(&style.color, 0.9)?;
        match style.mode {
            WaveformMode::Scope if !f.scope.is_empty() => {
                for channel in 0..2 {
                    let mut path = PathBuilder::stroke(px(style.line_width * scale));
                    for (i, pair) in f.scope.iter().enumerate() {
                        let x = wave[0] + i as f32 / (f.scope.len() - 1) as f32 * wave[2];
                        let center = wave[1] + wave[3] * (0.25 + channel as f32 * 0.5);
                        let y =
                            center - (pair[channel] * style.gain).clamp(-1., 1.) * wave[3] * 0.23;
                        let pt = point(px(x), px(y));
                        if i == 0 {
                            path.move_to(pt);
                        } else {
                            path.line_to(pt);
                        }
                    }
                    w.paint_path(path.build()?, ink);
                }
            }
            WaveformMode::Envelope if !f.waveform.is_empty() => {
                let step = wave[2] / f.waveform.len() as f32;
                for (i, peak) in f.waveform.iter().enumerate() {
                    let a = (peak.left * style.gain).min(1.) * wave[3] * 0.48;
                    let b = (peak.right * style.gain).min(1.) * wave[3] * 0.48;
                    if a + b < 0.01 * scale {
                        continue;
                    }
                    let mut ink = ink;
                    if i >= f.waveform.len() / 2 {
                        ink.a *= 0.4;
                    }
                    rectangle(
                        w,
                        [
                            wave[0] + i as f32 * step,
                            wave[1] + wave[3] / 2. - a,
                            step * 0.65,
                            a + b,
                        ],
                        ink,
                        step * 0.2,
                    );
                }
            }
            _ => {}
        }
    }
    if p.visual.spectrum && !f.spectrum.bars.is_empty() {
        let style = &p.visual.spectrum_style;
        let solid = color(&style.color, 0.95)?;
        let ink = match style.palette {
            SpectrumPalette::White => hsla(0., 0., 1., 0.92),
            SpectrumPalette::Solid => solid,
        };
        let step = spec[2] / f.spectrum.bars.len() as f32;
        let bar_width = step * (1. - style.gap);
        for (i, &level) in f.spectrum.bars.iter().enumerate() {
            let x = spec[0] + i as f32 * step;
            let height = spec[3] * level;
            if height > 0.1 * scale {
                rectangle(
                    w,
                    [x, spec[1] + spec[3] - height, bar_width, height],
                    ink,
                    (style.corner_radius * scale)
                        .min(bar_width / 2.)
                        .min(height / 2.),
                );
            }
            let peak = f.spectrum.peaks[i];
            if style.peak_caps && peak > 0.005 {
                let cap = (2. * scale).min(spec[3] * peak);
                // Small gap under caps, clamped inside the visualization box.
                let y = (spec[1] + spec[3] * (1. - peak) - cap - 2. * scale).max(spec[1]);
                rectangle(w, [x, y, bar_width, cap], ink, 0.);
            }
        }
    }
    Ok(())
}
