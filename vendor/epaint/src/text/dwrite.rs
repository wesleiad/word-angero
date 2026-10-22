//! Angerô: glyphs rasterized by DirectWrite (Windows' own text engine) instead of `vello_cpu`.
//!
//! egui draws text as grey-scale coverage without Windows' strong grid-fitting, which looks blurry
//! next to Office on 100 % screens. DirectWrite applies the font's TrueType hinting the way Windows
//! does and returns the same kind of 8-bit coverage, so only this step changes: layout, advances and
//! the atlas stay egui's. Any failure (unsupported font, variable-font instance…) returns `None` and
//! the caller falls back to egui's own rasterizer.
//!
//! GDI-classic grid-fitting also rounds each glyph's advance to whole pixels; [`gdi_advance_delta`]
//! gives the layout that same advance, otherwise the snapped shapes sit unevenly ("ST ELLA").
//!
//! `ANGERO_TEXTO` picks the rendering mode for comparisons: `gdi` (default: GDI-classic grid-fitting,
//! the sharpest), `natural`, `simetrico` (natural symmetric) or `egui` (off).

#![allow(unsafe_code)]

use std::cell::RefCell;
use std::collections::HashMap;

use windows::Win32::Graphics::DirectWrite::{
    DWRITE_FACTORY_TYPE_ISOLATED, DWRITE_FONT_FACE_TYPE, DWRITE_FONT_FILE_TYPE, DWRITE_FONT_METRICS,
    DWRITE_FONT_SIMULATIONS_NONE, DWRITE_GLYPH_METRICS,
    DWRITE_GLYPH_OFFSET, DWRITE_GLYPH_RUN, DWRITE_GRID_FIT_MODE_ENABLED, DWRITE_MEASURING_MODE,
    DWRITE_MEASURING_MODE_GDI_CLASSIC, DWRITE_MEASURING_MODE_NATURAL, DWRITE_RENDERING_MODE1,
    DWRITE_RENDERING_MODE1_GDI_CLASSIC, DWRITE_RENDERING_MODE1_NATURAL, DWRITE_RENDERING_MODE1_NATURAL_SYMMETRIC,
    DWRITE_TEXT_ANTIALIAS_MODE_GRAYSCALE, DWRITE_TEXTURE_ALIASED_1x1, DWriteCreateFactory, IDWriteFactory5,
    IDWriteFontFace, IDWriteInMemoryFontFileLoader,
};

/// A glyph's coverage: `width × height` bytes, placed at `(left, top)` pixels from the pen position
/// on the baseline (y down).
pub(crate) struct Glyph {
    pub left: i32,
    pub top: i32,
    pub width: usize,
    pub height: usize,
    pub coverage: Vec<u8>,
}

#[derive(Clone, Copy, PartialEq)]
enum Modo {
    Gdi,
    Natural,
    Simetrico,
    Desligado,
}

fn modo() -> Modo {
    static MODO: std::sync::OnceLock<Modo> = std::sync::OnceLock::new();
    *MODO.get_or_init(|| match std::env::var("ANGERO_TEXTO").unwrap_or_default().to_ascii_lowercase().as_str() {
        "natural" => Modo::Natural,
        "simetrico" => Modo::Simetrico,
        "egui" | "desligado" => Modo::Desligado,
        _ => Modo::Gdi,
    })
}

struct Motor {
    factory: IDWriteFactory5,
    loader: IDWriteInMemoryFontFileLoader,
    /// (address and length of the font bytes, face index) → face; `None` = DirectWrite can't read it.
    faces: HashMap<(usize, usize, u32), Option<IDWriteFontFace>>,
    /// (font bytes address, face index, glyph, size bits) → advance correction in pixels.
    avancos: HashMap<(usize, u32, u16, u32), Option<f32>>,
}

thread_local! {
    // COM objects stay on the thread that made them; egui lays text out on the UI thread.
    static MOTOR: RefCell<Option<Option<Motor>>> = const { RefCell::new(None) };
}

fn novo_motor() -> Option<Motor> {
    unsafe {
        let factory: IDWriteFactory5 = DWriteCreateFactory(DWRITE_FACTORY_TYPE_ISOLATED).ok()?;
        let loader = factory.CreateInMemoryFontFileLoader().ok()?;
        factory.RegisterFontFileLoader(&loader).ok()?;
        Some(Motor { factory, loader, faces: HashMap::new(), avancos: HashMap::new() })
    }
}

impl Motor {
    fn face(&mut self, bytes: &[u8], index: u32) -> Option<IDWriteFontFace> {
        let key = (bytes.as_ptr() as usize, bytes.len(), index);
        if let Some(f) = self.faces.get(&key) {
            return f.clone();
        }
        let face = unsafe {
            // No owner object: DirectWrite keeps its own copy of the bytes.
            (|| {
                let file = self
                    .loader
                    .CreateInMemoryFontFileReference(
                        &self.factory,
                        bytes.as_ptr().cast(),
                        u32::try_from(bytes.len()).ok()?,
                        None::<&windows::core::IUnknown>,
                    )
                    .ok()?;
                let mut supported = windows::core::BOOL(0);
                let mut file_type = DWRITE_FONT_FILE_TYPE::default();
                let mut face_type = DWRITE_FONT_FACE_TYPE::default();
                let mut faces = 0u32;
                file.Analyze(&mut supported, &mut file_type, Some(&mut face_type), &mut faces).ok()?;
                if !supported.as_bool() || index >= faces {
                    return None;
                }
                self.factory.CreateFontFace(face_type, &[Some(file)], index, DWRITE_FONT_SIMULATIONS_NONE).ok()
            })()
        };
        self.faces.insert(key, face.clone());
        face
    }
}

/// How many pixels GDI-classic grid-fitting adds to (or takes from) the design advance of
/// `glyph_id` at `size_px` pixels per em; `None` outside the GDI mode or when DirectWrite can't tell.
/// Added to the shaper's advance, so kerning is kept.
pub(crate) fn gdi_advance_delta(bytes: &[u8], index: u32, glyph_id: u16, size_px: f32) -> Option<f32> {
    if modo() != Modo::Gdi || !(size_px > 0.0 && size_px < 1000.0) {
        return None;
    }
    MOTOR.with(|m| {
        let mut m = m.borrow_mut();
        let motor = m.get_or_insert_with(novo_motor).as_mut()?;
        let key = (bytes.as_ptr() as usize, index, glyph_id, size_px.to_bits());
        if let Some(v) = motor.avancos.get(&key) {
            return *v;
        }
        let face = motor.face(bytes, index)?;
        let v = unsafe {
            (|| {
                let mut fm = DWRITE_FONT_METRICS::default();
                face.GetMetrics(&mut fm);
                if fm.designUnitsPerEm == 0 {
                    return None;
                }
                let ids = [glyph_id];
                let mut gdi = [DWRITE_GLYPH_METRICS::default()];
                let mut design = [DWRITE_GLYPH_METRICS::default()];
                // The glyph is drawn at whole pixels per em (see `rasterize`): its GDI advance at that size,
                // minus the design advance at the layout's size the shaper used.
                let em = size_px.round().max(1.0);
                face.GetGdiCompatibleGlyphMetrics(em, 1.0, None, false, ids.as_ptr(), 1, gdi.as_mut_ptr(), false).ok()?;
                face.GetDesignGlyphMetrics(ids.as_ptr(), 1, design.as_mut_ptr(), false).ok()?;
                let upem = fm.designUnitsPerEm as f32;
                Some(gdi[0].advanceWidth as f32 * em / upem - design[0].advanceWidth as f32 * size_px / upem)
            })()
        };
        motor.avancos.insert(key, v);
        v
    })
}

/// Rasterizes glyph `glyph_id` of face `index` in `bytes` at `size_px` pixels per em, with the pen
/// `x_offset` pixels right of the whole-pixel origin (egui's sub-pixel bin).
pub(crate) fn rasterize(bytes: &[u8], index: u32, glyph_id: u16, size_px: f32, x_offset: f32) -> Option<Glyph> {
    let modo = modo();
    if modo == Modo::Desligado || !(size_px > 0.0 && size_px < 1000.0) {
        return None;
    }
    MOTOR.with(|m| {
        let mut m = m.borrow_mut();
        let motor = m.get_or_insert_with(novo_motor).as_mut()?;
        let face = motor.face(bytes, index)?;
        let (rendering, measuring): (DWRITE_RENDERING_MODE1, DWRITE_MEASURING_MODE) = match modo {
            Modo::Gdi => (DWRITE_RENDERING_MODE1_GDI_CLASSIC, DWRITE_MEASURING_MODE_GDI_CLASSIC),
            Modo::Natural => (DWRITE_RENDERING_MODE1_NATURAL, DWRITE_MEASURING_MODE_NATURAL),
            _ => (DWRITE_RENDERING_MODE1_NATURAL_SYMMETRIC, DWRITE_MEASURING_MODE_NATURAL),
        };
        // GDI-classic hints at whole pixels per em (as GDI does); the advances below use the same size.
        let em = if modo == Modo::Gdi { size_px.round().max(1.0) } else { size_px };
        let indices = [glyph_id];
        let advances = [0.0f32];
        let offsets = [DWRITE_GLYPH_OFFSET::default()];
        let run = DWRITE_GLYPH_RUN {
            fontFace: std::mem::ManuallyDrop::new(Some(face)),
            fontEmSize: em,
            glyphCount: 1,
            glyphIndices: indices.as_ptr(),
            glyphAdvances: advances.as_ptr(),
            glyphOffsets: offsets.as_ptr(),
            isSideways: false.into(),
            bidiLevel: 0,
        };
        let resultado = unsafe {
            (|| {
                let analysis = motor
                    .factory
                    .CreateGlyphRunAnalysis(
                        &run,
                        None,
                        rendering,
                        measuring,
                        DWRITE_GRID_FIT_MODE_ENABLED,
                        DWRITE_TEXT_ANTIALIAS_MODE_GRAYSCALE,
                        x_offset,
                        0.0,
                    )
                    .ok()?;
                let r = analysis.GetAlphaTextureBounds(DWRITE_TEXTURE_ALIASED_1x1).ok()?;
                let (width, height) = ((r.right - r.left).max(0) as usize, (r.bottom - r.top).max(0) as usize);
                let mut coverage = vec![0u8; width * height];
                if width > 0 && height > 0 {
                    analysis.CreateAlphaTexture(DWRITE_TEXTURE_ALIASED_1x1, &r, &mut coverage).ok()?;
                }
                Some(Glyph { left: r.left, top: r.top, width, height, coverage })
            })()
        };
        drop(std::mem::ManuallyDrop::into_inner(run.fontFace));
        resultado
    })
}
