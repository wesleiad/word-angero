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

/// RAM física total, em bytes (só Windows). Office Angerô: decide o modo leve.
#[cfg(target_os = "windows")]
pub fn ram_total() -> Option<u64> {
    #[repr(C)]
    struct MemoryStatusEx {
        length: u32,
        memory_load: u32,
        total_phys: u64,
        avail_phys: u64,
        total_page_file: u64,
        avail_page_file: u64,
        total_virtual: u64,
        avail_virtual: u64,
        avail_extended_virtual: u64,
    }
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GlobalMemoryStatusEx(buffer: *mut MemoryStatusEx) -> i32;
    }
    let mut m = MemoryStatusEx {
        length: std::mem::size_of::<MemoryStatusEx>() as u32,
        memory_load: 0,
        total_phys: 0,
        avail_phys: 0,
        total_page_file: 0,
        avail_page_file: 0,
        total_virtual: 0,
        avail_virtual: 0,
        avail_extended_virtual: 0,
    };
    // SAFETY: `m` é um MEMORYSTATUSEX válido com `length` preenchido, como a API pede.
    (unsafe { GlobalMemoryStatusEx(&mut m) } != 0).then_some(m.total_phys)
}

#[cfg(not(target_os = "windows"))]
pub fn ram_total() -> Option<u64> {
    None
}
