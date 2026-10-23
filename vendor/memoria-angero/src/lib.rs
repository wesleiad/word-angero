//! Office Angerô: chamadas ao Windows que precisam de `unsafe`, isoladas aqui porque o workspace
//! proíbe `unsafe` (`forbid(unsafe_code)`).

/// Área livre do monitor principal (sem a barra de tarefas), em pontos lógicos: (x, y, largura, altura).
/// Office Angerô: usada para a janela abrir cabendo na tela em notebooks e telas com zoom de 125-150 %.
#[cfg(target_os = "windows")]
pub fn area_trabalho() -> Option<(f32, f32, f32, f32)> {
    #[repr(C)]
    struct Retangulo {
        esquerda: i32,
        topo: i32,
        direita: i32,
        baixo: i32,
    }
    #[link(name = "user32")]
    unsafe extern "system" {
        fn SystemParametersInfoW(acao: u32, param: u32, dados: *mut Retangulo, ini: u32) -> i32;
        fn GetDpiForSystem() -> u32;
    }
    const SPI_GETWORKAREA: u32 = 0x0030;
    let mut r = Retangulo { esquerda: 0, topo: 0, direita: 0, baixo: 0 };
    // SAFETY: `r` é um RECT válido; as duas funções só leem configurações do sistema.
    let (ok, dpi) = unsafe { (SystemParametersInfoW(SPI_GETWORKAREA, 0, &mut r, 0), GetDpiForSystem()) };
    if ok == 0 || dpi == 0 || r.direita <= r.esquerda || r.baixo <= r.topo {
        return None;
    }
    // processo com ou sem reconhecimento de DPI: pixels e DPI vêm na mesma escala, a divisão dá pontos lógicos
    let k = dpi as f32 / 96.0;
    Some((
        r.esquerda as f32 / k,
        r.topo as f32 / k,
        (r.direita - r.esquerda) as f32 / k,
        (r.baixo - r.topo) as f32 / k,
    ))
}

#[cfg(not(target_os = "windows"))]
pub fn area_trabalho() -> Option<(f32, f32, f32, f32)> {
    None
}
