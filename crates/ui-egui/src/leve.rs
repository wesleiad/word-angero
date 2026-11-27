//! Office Angerô: modo leve para computadores fracos (vídeo básico, até 8 GB de RAM ou até 4 núcleos).
//!
//! Num vídeo sem driver próprio ("Microsoft Basic Render Driver") cada quadro é desenhado pelo
//! processador, então o que mais pesa é a QUANTIDADE de quadros. O modo leve corta os quadros que
//! não trazem informação: animações (o cursor deslizando ao digitar, esmaecimentos), rolagem suave e o fundo Mica da
//! barra de título.
//!
//! `ANGERO_LEVE=1` liga e `ANGERO_LEVE=0` desliga à força; sem a variável, decide pelo hardware.

use std::sync::atomic::{AtomicBool, Ordering};

static LEVE: AtomicBool = AtomicBool::new(false);

/// O modo leve está ligado?
pub fn ativo() -> bool {
    LEVE.load(Ordering::Relaxed)
}

pub fn definir(ligado: bool) {
    LEVE.store(ligado, Ordering::Relaxed);
}

/// A escolha forçada pelo ambiente (`ANGERO_LEVE`), se houver.
pub fn forcado() -> Option<bool> {
    match std::env::var("ANGERO_LEVE").ok()?.trim().to_ascii_lowercase().as_str() {
        "1" | "sim" | "leve" | "on" => Some(true),
        "0" | "nao" | "não" | "normal" | "off" => Some(false),
        _ => None,
    }
}

/// Decide pelo hardware: `ram` em bytes, `nucleos` lógicos, `desenho_por_software` quando o vídeo
/// em uso é o adaptador por software (WARP / llvmpipe / GDI Generic).
pub fn decidir(ram: Option<u64>, nucleos: usize, desenho_por_software: bool) -> bool {
    if let Some(f) = forcado() {
        return f;
    }
    // "8 GB" aparece como 7,7–8,0 GB (parte fica com o vídeo integrado): limite com folga.
    desenho_por_software || nucleos <= 4 || ram.is_some_and(|b| b <= 8_600 * 1024 * 1024)
}

/// Aplica no contexto o que é de estilo: sem animações nem rolagem suave. Chamado a cada quadro
/// (barato), porque a troca de tema refaz o estilo.
pub fn aplicar_estilo(ctx: &egui::Context) {
    if !ativo() || ctx.global_style().animation_time == 0.0 {
        return;
    }
    ctx.all_styles_mut(|s| {
        s.animation_time = 0.0;
        s.scroll_animation = egui::style::ScrollAnimation::none();
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decide_pelo_hardware() {
        // Sem ANGERO_LEVE no ambiente dos testes.
        if forcado().is_some() {
            return;
        }
        let gb = |n: u64| Some(n << 30);
        assert!(decidir(gb(8), 8, false));
        assert!(decidir(gb(16), 4, false));
        assert!(decidir(gb(16), 12, true));
        assert!(!decidir(gb(16), 12, false));
        assert!(!decidir(None, 8, false));
    }
}
