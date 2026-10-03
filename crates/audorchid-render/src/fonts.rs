use anyhow::{Context, Result, ensure};
use audorchid_core::Project;
use gpui::{Bounds, DevicePixels, LineLayout, PlatformTextSystem, RenderGlyphParams, point};
use gpui_wgpu::CosmicTextSystem;
use std::path::Path;
use std::{cell::RefCell, collections::HashMap, rc::Rc, sync::Arc};

pub(crate) struct Fonts {
    pub latin_bytes: Vec<u8>,
    pub cjk_bytes: Option<Vec<u8>>,
    pub latin_family: String,
    pub cjk_family: Option<String>,
}
#[derive(Clone)]
pub(crate) struct FontNames {
    pub latin: String,
    pub cjk: Option<String>,
    raster: Arc<CosmicTextSystem>,
    bounds: Rc<RefCell<HashMap<RenderGlyphParams, Bounds<DevicePixels>>>>,
}
impl Fonts {
    pub fn names(&self, raster: Arc<CosmicTextSystem>) -> FontNames {
        FontNames {
            latin: self.latin_family.clone(),
            cjk: self.cjk_family.clone(),
            raster,
            bounds: Rc::new(RefCell::new(HashMap::new())),
        }
    }
}
impl FontNames {
    /// Actual ink bounds relative to the baseline, including fallback runs.
    /// A font's line metrics alone do not contain every mixed-font glyph.
    pub fn ink_bounds(&self, line: &LineLayout, dpi: f32) -> Result<[f32; 4]> {
        let mut extent = [
            f32::INFINITY,
            f32::INFINITY,
            f32::NEG_INFINITY,
            f32::NEG_INFINITY,
        ];
        for run in &line.runs {
            for glyph in &run.glyphs {
                let params = RenderGlyphParams {
                    font_id: run.font_id,
                    glyph_id: glyph.id,
                    font_size: line.font_size,
                    subpixel_variant: point(0, 0),
                    scale_factor: dpi,
                    is_emoji: glyph.is_emoji,
                    subpixel_rendering: false,
                    dilation: 0,
                    blur_radius: 0,
                };
                let mut cache = self.bounds.borrow_mut();
                let b = match cache.entry(params.clone()) {
                    std::collections::hash_map::Entry::Occupied(entry) => *entry.get(),
                    std::collections::hash_map::Entry::Vacant(entry) => {
                        *entry.insert(self.raster.glyph_raster_bounds(&params)?)
                    }
                };
                if b.size.width.0 == 0 || b.size.height.0 == 0 {
                    continue;
                }
                let x = f32::from(glyph.position.x) + b.origin.x.0 as f32 / dpi;
                let y = f32::from(glyph.position.y) + b.origin.y.0 as f32 / dpi;
                extent[0] = extent[0].min(x);
                extent[1] = extent[1].min(y);
                extent[2] = extent[2].max(x + b.size.width.0 as f32 / dpi);
                extent[3] = extent[3].max(y + b.size.height.0 as f32 / dpi);
            }
        }
        Ok(if extent[0].is_finite() {
            extent
        } else {
            [0.; 4]
        })
    }
}
pub fn validate_fonts(p: &Project) -> Result<()> {
    load(p).map(|_| ())
}
fn read(path: Option<&Path>, bundled: &[u8]) -> Result<(Vec<u8>, String)> {
    let label = path
        .map(|p| p.display().to_string())
        .unwrap_or("embedded font".into());
    let bytes = match path {
        Some(path) => std::fs::read(path).with_context(|| format!("font: {label}"))?,
        None => bundled.to_vec(),
    };
    let face = ttf_parser::Face::parse(&bytes, 0)
        .with_context(|| format!("cannot parse font: {label}"))?;
    // fontdb prefers the typographic family (name 16), not Windows' legacy
    // four-style family (name 1). Dream Han's W10 suffix appears only in name 1.
    let family = [16, 1]
        .into_iter()
        .find_map(|id| {
            face.names()
                .into_iter()
                .filter(|n| n.name_id == id)
                .filter_map(|n| n.to_string())
                .find(|s| s.is_ascii())
                .or_else(|| {
                    face.names()
                        .into_iter()
                        .filter(|n| n.name_id == id)
                        .find_map(|n| n.to_string())
                })
        })
        .context("font has no family name")?;
    Ok((bytes, family))
}
pub(crate) fn is_cjk(c: char) -> bool {
    matches!(c as u32,0x2e80..=0x9fff|0xac00..=0xd7af|0xf900..=0xfaff|0xfe30..=0xfe4f|0xff00..=0xffef|0x20000..=0x323af)
}
pub(crate) fn load(p: &Project) -> Result<Fonts> {
    let (latin_bytes, latin_family) = read(
        p.fonts.latin.as_deref(),
        include_bytes!("../../../assets/fonts/Roboto-Regular.ttf"),
    )?;
    let cjk = Some(read(
        p.fonts.cjk.as_deref(),
        include_bytes!("../../../assets/fonts/DreamHanSansCN-W10.ttf"),
    )?);
    let latin = ttf_parser::Face::parse(&latin_bytes, 0)?;
    let cjk_face = cjk
        .as_ref()
        .map(|(b, _)| ttf_parser::Face::parse(b, 0))
        .transpose()?;
    for text in p
        .tracks
        .iter()
        .flat_map(|t| [t.title.as_str(), t.artist.as_str()])
        .chain(["AUDORCHID 0123456789: / ."])
    {
        for c in text.chars().filter(|c| !c.is_whitespace()) {
            let face = if is_cjk(c) {
                cjk_face.as_ref().context("CJK text requires fonts.cjk")?
            } else {
                &latin
            };
            ensure!(
                face.glyph_index(c).is_some(),
                "font is missing character '{c}' (U+{:04X}); choose a font with this glyph",
                c as u32
            );
        }
    }
    let (cjk_bytes, cjk_family) = match cjk {
        Some((b, n)) => (Some(b), Some(n)),
        None => (None, None),
    };
    Ok(Fonts {
        latin_bytes,
        latin_family,
        cjk_bytes,
        cjk_family,
    })
}
