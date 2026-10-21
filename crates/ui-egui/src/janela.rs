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
