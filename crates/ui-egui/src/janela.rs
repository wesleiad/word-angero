//! Angerô: on Windows the window has no system title bar (like the official Office); these are
//! the Minimize / Maximize / Close buttons drawn at the right end of the app's own title bar.

use egui::{Color32, Rect, Sense, Stroke, StrokeKind, Ui, pos2, vec2};

/// Width of each caption button, as in Office.
pub const LARGURA_BOTAO: f32 = 46.0;

/// The system title bar is replaced by the app's own (Windows only).
pub const SEM_BARRA_DO_SISTEMA: bool = cfg!(windows);

#[derive(Clone, Copy)]
enum Botao {
    Fechar,
    Maximizar,
    Minimizar,
}

/// Draws the three buttons flush with the top-right corner of `barra` in `cor`; returns the width
/// they take (0 when the system title bar is shown).
pub fn botoes(ui: &mut Ui, barra: Rect, cor: Color32) -> f32 {
    if !SEM_BARRA_DO_SISTEMA {
        return 0.0;
    }
    let maximizada = ui.ctx().input(|i| i.viewport().maximized.unwrap_or(false));
    let mut x = barra.max.x;
    for (i, botao) in [Botao::Fechar, Botao::Maximizar, Botao::Minimizar].into_iter().enumerate() {
        let r = Rect::from_min_max(pos2(x - LARGURA_BOTAO, barra.min.y), pos2(x, barra.max.y));
        x -= LARGURA_BOTAO;
        let resp = ui.interact(r, ui.id().with(("angero_botao_janela", i)), Sense::click());
        let mut traco = cor;
        if resp.hovered() {
            if matches!(botao, Botao::Fechar) {
                ui.painter().rect_filled(r, 0.0, Color32::from_rgb(0xC4, 0x2B, 0x1C));
                traco = Color32::WHITE;
            } else {
                ui.painter().rect_filled(r, 0.0, cor.gamma_multiply(0.14));
            }
        }
        let c = r.center().round();
        let s = 5.0;
        let st = Stroke::new(1.0, traco);
        let p = ui.painter();
        let dica = match botao {
            Botao::Minimizar => {
                p.line_segment([pos2(c.x - s, c.y + 0.5), pos2(c.x + s, c.y + 0.5)], st);
                "Minimizar"
            }
            Botao::Maximizar if maximizada => {
                p.rect_stroke(Rect::from_min_size(pos2(c.x - s, c.y - s + 2.0), vec2(2.0 * s - 2.0, 2.0 * s - 2.0)), 0.0, st, StrokeKind::Inside);
                p.line_segment([pos2(c.x - s + 2.0, c.y - s), pos2(c.x + s, c.y - s)], st);
                p.line_segment([pos2(c.x + s, c.y - s), pos2(c.x + s, c.y + s - 2.0)], st);
                "Restaurar"
            }
            Botao::Maximizar => {
                p.rect_stroke(Rect::from_center_size(c, vec2(2.0 * s, 2.0 * s)), 0.0, st, StrokeKind::Inside);
                "Maximizar"
            }
            Botao::Fechar => {
                p.line_segment([pos2(c.x - s, c.y - s), pos2(c.x + s, c.y + s)], st);
                p.line_segment([pos2(c.x - s, c.y + s), pos2(c.x + s, c.y - s)], st);
                "Fechar"
            }
        };
        if resp.on_hover_text(dica).clicked() {
            let comando = match botao {
                Botao::Minimizar => egui::ViewportCommand::Minimized(true),
                Botao::Maximizar => egui::ViewportCommand::Maximized(!maximizada),
                // Goes through the app's close handling, so it still asks to save changes.
                Botao::Fechar => egui::ViewportCommand::Close,
            };
            ui.ctx().send_viewport_cmd(comando);
        }
    }
    3.0 * LARGURA_BOTAO
}

/// Opacity of the bar's colour over the wallpaper: enough to keep the text readable, with the
/// wallpaper's tone still showing through (Windows 11 Mica).
const VEU: u8 = 165;

/// Windows 11 "Mica" look for the title bar: the desktop wallpaper, small and heavily blurred, cut to
/// the part of the screen behind `barra`, under a veil of the bar's colour `cor` — what DWM does for
/// Mica. Drawn by the app itself so it works on any GPU and driver (a truly transparent window needs
/// DirectComposition); without a wallpaper picture (solid colour desktop) the bar keeps its colour.
pub fn fundo_mica(ui: &Ui, barra: Rect, cor: Color32) {
    if !SEM_BARRA_DO_SISTEMA {
        return;
    }
    let Some(tex) = textura_papel(ui.ctx()) else { return };
    let (janela, monitor) = ui.ctx().input(|i| (i.viewport().inner_rect, i.viewport().monitor_size));
    let (Some(janela), Some(monitor)) = (janela, monitor) else { return };
    if monitor.x < 1.0 || monitor.y < 1.0 {
        return;
    }
    // Where the bar is on its monitor; each monitor shows the whole wallpaper, scaled to fill it.
    let x = janela.min.x.rem_euclid(monitor.x) + barra.min.x;
    let y = janela.min.y.rem_euclid(monitor.y) + barra.min.y;
    let [iw, ih] = tex.size().map(|v| v as f32);
    let escala = (monitor.x / iw).max(monitor.y / ih);
    let (lw, lh) = (iw * escala, ih * escala);
    let (ox, oy) = ((monitor.x - lw) / 2.0, (monitor.y - lh) / 2.0);
    let uv = Rect::from_min_max(
        pos2((x - ox) / lw, (y - oy) / lh),
        pos2((x + barra.width() - ox) / lw, (y + barra.height() - oy) / lh),
    );
    let p = ui.painter();
    p.image(tex.id(), barra, uv, Color32::WHITE);
    p.rect_filled(barra, 0.0, Color32::from_rgba_unmultiplied(cor.r(), cor.g(), cor.b(), VEU));
}

/// The blurred wallpaper, decoded once on a background thread (the first frames show the plain bar).
static PAPEL: std::sync::OnceLock<std::sync::Mutex<Option<Option<egui::ColorImage>>>> = std::sync::OnceLock::new();

fn textura_papel(ctx: &egui::Context) -> Option<egui::TextureHandle> {
    let id = egui::Id::new("angero_mica");
    if let Some(t) = ctx.data(|d| d.get_temp::<egui::TextureHandle>(id)) {
        return Some(t);
    }
    let mut primeira = false;
    let estado = PAPEL.get_or_init(|| {
        primeira = true;
        std::sync::Mutex::new(None)
    });
    if primeira {
        let c = ctx.clone();
        std::thread::spawn(move || {
            let img = carregar_papel();
            if let Some(m) = PAPEL.get()
                && let Ok(mut g) = m.lock()
            {
                *g = Some(img);
            }
            c.request_repaint();
        });
        return None;
    }
    let mut g = estado.lock().ok()?;
    match g.take() {
        Some(Some(img)) => {
            let t = ctx.load_texture("angero_mica", img, egui::TextureOptions::LINEAR);
            ctx.data_mut(|d| d.insert_temp(id, t.clone()));
            Some(t)
        }
        Some(None) => {
            *g = Some(None); // no wallpaper picture: keep the plain bar
            None
        }
        None => None, // still decoding
    }
}

/// The Windows wallpaper (the copy Windows keeps for the desktop), 64 px wide and blurred.
fn carregar_papel() -> Option<egui::ColorImage> {
    let pasta = std::env::var_os("APPDATA")?;
    let bytes = std::fs::read(std::path::Path::new(&pasta).join(r"Microsoft\Windows\Themes\TranscodedWallpaper")).ok()?;
    let img = image::load_from_memory(&bytes).ok()?.to_rgb8();
    let w = 64u32;
    let h = ((img.height() as f32 / img.width().max(1) as f32) * w as f32).round().max(1.0) as u32;
    let pequena = image::imageops::resize(&img, w, h, image::imageops::FilterType::Triangle);
    let borrada = image::imageops::blur(&pequena, 2.5);
    Some(egui::ColorImage::from_rgb([w as usize, h as usize], borrada.as_raw()))
}

/// Thickness of the grab strip along each window edge, in points.
const BORDA: f32 = 5.0;

/// Resizing by the window edges and corners. Without the system frame Windows has no edge to grab
/// (the client area covers the whole window), so the app does it: a thin strip on each side, above
/// everything else, that shows the resize cursor and hands the drag to the system.
pub fn bordas(ctx: &egui::Context) {
    use egui::{CursorIcon, ResizeDirection as D};
    if !SEM_BARRA_DO_SISTEMA {
        return;
    }
    let (maximizada, cheia) = ctx.input(|i| (i.viewport().maximized.unwrap_or(false), i.viewport().fullscreen.unwrap_or(false)));
    if maximizada || cheia {
        return;
    }
    let r = ctx.content_rect();
    let (b, c) = (BORDA, 2.0 * BORDA);
    let zonas = [
        (D::NorthWest, CursorIcon::ResizeNorthWest, Rect::from_min_size(r.left_top(), vec2(c, c))),
        (D::NorthEast, CursorIcon::ResizeNorthEast, Rect::from_min_size(pos2(r.right() - c, r.top()), vec2(c, c))),
        (D::SouthWest, CursorIcon::ResizeSouthWest, Rect::from_min_size(pos2(r.left(), r.bottom() - c), vec2(c, c))),
        (D::SouthEast, CursorIcon::ResizeSouthEast, Rect::from_min_size(pos2(r.right() - c, r.bottom() - c), vec2(c, c))),
        (D::North, CursorIcon::ResizeNorth, Rect::from_min_max(pos2(r.left() + c, r.top()), pos2(r.right() - c, r.top() + b))),
        (D::South, CursorIcon::ResizeSouth, Rect::from_min_max(pos2(r.left() + c, r.bottom() - b), pos2(r.right() - c, r.bottom()))),
        (D::West, CursorIcon::ResizeWest, Rect::from_min_max(pos2(r.left(), r.top() + c), pos2(r.left() + b, r.bottom() - c))),
        (D::East, CursorIcon::ResizeEast, Rect::from_min_max(pos2(r.right() - b, r.top() + c), pos2(r.right(), r.bottom() - c))),
    ];
    // Kept above the other Foreground areas (menus, the start screen), which are created later.
    let id = egui::Id::new("angero_bordas_janela");
    ctx.move_to_top(egui::LayerId::new(egui::Order::Foreground, id));
    egui::Area::new(id)
        .order(egui::Order::Foreground)
        .fixed_pos(r.min)
        .interactable(true)
        .show(ctx, |ui| {
            for (i, (direcao, cursor, zona)) in zonas.into_iter().enumerate() {
                let resp = ui.interact(zona, ui.id().with(("borda", i)), Sense::drag());
                if resp.hovered() || resp.dragged() {
                    ctx.set_cursor_icon(cursor);
                }
                // At the press itself, not after egui's drag threshold: the system takes over the
                // mouse from where it went down.
                if resp.is_pointer_button_down_on() && ctx.input(|i| i.pointer.primary_pressed()) {
                    ctx.send_viewport_cmd(egui::ViewportCommand::BeginResize(direcao));
                }
            }
        });
}
