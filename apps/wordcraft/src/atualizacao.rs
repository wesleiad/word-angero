//! Office Angerô: atualização automática pela aba Releases do GitHub.
//!
//! Cada versão publicada tem dois arquivos de nome fixo (links "releases/latest/download/...", que
//! não usam a API do GitHub, então não há limite de consultas por computador):
//!   versao.txt                      ex.: 0.8.0.42
//!   <exe>-windows-x64.zip           o programa (exe, cli, licenças e versao.txt)
//! Ao abrir (e a cada 6 horas), em segundo plano e com prioridade baixa, compara com o `versao.txt`
//! ao lado do .exe. Se houver versão nova, baixa o zip e troca os arquivos: o Windows deixa renomear
//! um .exe em uso, então o atual vira `.old` (apagado na próxima abertura) e a versão nova vale da
//! próxima vez que o programa abrir. Só age em cópias instaladas (com `versao.txt` ao lado do .exe).
//! Baixa com o curl.exe do Windows (10/11). `OFFICE_ANGERO_SEM_ATUALIZACAO=1` desliga.
//!
//! Arquivo copiado de Office-Angero/programas/atualizacao.rs — mudar lá e copiar para os 3 programas.

#[cfg(target_os = "windows")]
mod windows {
    use std::io::Read;
    use std::os::windows::process::CommandExt;
    use std::path::{Path, PathBuf};
    use std::process::Command;
    use std::time::Duration;

    const SEM_JANELA: u32 = 0x0800_0000; // CREATE_NO_WINDOW
    const ESPERA_INICIAL: Duration = Duration::from_secs(30);
    const INTERVALO: Duration = Duration::from_secs(6 * 3600);

    fn versao(texto: &str) -> Vec<u64> {
        texto.trim().trim_start_matches('v').split('.').map(|p| p.parse().unwrap_or(0)).collect()
    }

    /// Baixa `url` para `destino` com o curl.exe do Windows. `--ssl-no-revoke`: proxies com inspeção
    /// HTTPS (Netskope) quebram a checagem de revogação do certificado.
    fn baixar(url: &str, destino: &Path, max_seg: u32) -> bool {
        let curl = std::env::var_os("SystemRoot").map(|r| PathBuf::from(r).join("System32").join("curl.exe"));
        let Some(curl) = curl.filter(|c| c.is_file()) else { return false };
        Command::new(curl)
            .args(["-sSfL", "--ssl-no-revoke", "--max-time", &max_seg.to_string(), "-o"])
            .arg(destino)
            .arg(url)
            .creation_flags(SEM_JANELA)
            .status()
            .is_ok_and(|s| s.success())
    }

    /// Troca `alvo` por `novo`: o atual (mesmo em uso) vira `.old`.
    fn trocar(alvo: &Path, novo: &[u8]) -> std::io::Result<()> {
        let tmp = alvo.with_extension("novo");
        std::fs::write(&tmp, novo)?;
        if alvo.exists() {
            let velho = PathBuf::from(format!("{}.old", alvo.display()));
            let _ = std::fs::remove_file(&velho);
            std::fs::rename(alvo, &velho)?;
        }
        std::fs::rename(&tmp, alvo)
    }

    fn uma_vez(pasta: &Path, repo: &str, exe: &str) -> Result<String, String> {
        let local = std::fs::read_to_string(pasta.join("versao.txt")).map_err(|e| e.to_string())?;
        let tmp = std::env::temp_dir().join(format!("office-angero-{exe}"));
        std::fs::create_dir_all(&tmp).map_err(|e| e.to_string())?;
        let base = format!("https://github.com/wesleiad/{repo}/releases/latest/download");
        let arq_versao = tmp.join("versao.txt");
        if !baixar(&format!("{base}/versao.txt"), &arq_versao, 30) {
            return Err("sem conexão com o GitHub".into());
        }
        let remota = std::fs::read_to_string(&arq_versao).map_err(|e| e.to_string())?;
        if versao(&remota) <= versao(&local) {
            return Ok(format!("atualizado ({})", local.trim()));
        }
        let arq_zip = tmp.join(format!("{exe}-windows-x64.zip"));
        if !baixar(&format!("{base}/{exe}-windows-x64.zip"), &arq_zip, 900) {
            return Err("download da versão nova falhou".into());
        }
        let mut z = zip::ZipArchive::new(std::fs::File::open(&arq_zip).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
        // tudo lido antes de trocar qualquer arquivo: zip incompleto não deixa o programa pela metade
        let mut arquivos = Vec::new();
        for i in 0..z.len() {
            let mut f = z.by_index(i).map_err(|e| e.to_string())?;
            if f.is_dir() {
                continue;
            }
            let Some(nome) = f.enclosed_name().and_then(|p| p.file_name().map(PathBuf::from)) else { continue };
            let mut dados = Vec::new();
            f.read_to_end(&mut dados).map_err(|e| e.to_string())?;
            arquivos.push((nome, dados));
        }
        if !arquivos.iter().any(|(n, _)| n == Path::new(&format!("{exe}.exe"))) {
            return Err("zip sem o programa".into());
        }
        // versao.txt por último: se algo falhar no meio, a próxima rodada tenta de novo
        arquivos.sort_by_key(|(n, _)| n == Path::new("versao.txt"));
        for (nome, dados) in &arquivos {
            trocar(&pasta.join(nome), dados).map_err(|e| format!("{}: {e}", nome.display()))?;
        }
        let _ = std::fs::remove_file(&arq_zip);
        Ok(format!("atualizado de {} para {} (vale ao reabrir)", local.trim(), remota.trim()))
    }

    pub fn iniciar(repo: &'static str, exe: &'static str) {
        if std::env::var_os("OFFICE_ANGERO_SEM_ATUALIZACAO").is_some() {
            return;
        }
        let Some(pasta) = std::env::current_exe().ok().and_then(|p| p.parent().map(Path::to_path_buf)) else { return };
        // restos da troca anterior (o .exe antigo só pode ser apagado depois que o programa fechou)
        for entrada in std::fs::read_dir(&pasta).into_iter().flatten().flatten() {
            let nome = entrada.file_name();
            let nome = nome.to_string_lossy();
            if nome.ends_with(".old") || nome.ends_with(".novo") {
                let _ = std::fs::remove_file(entrada.path());
            }
        }
        if !pasta.join("versao.txt").is_file() {
            return; // cópia de desenvolvimento
        }
        let _ = std::thread::Builder::new().name("atualizacao".into()).spawn(move || {
            std::thread::sleep(ESPERA_INICIAL);
            loop {
                match uma_vez(&pasta, repo, exe) {
                    Ok(m) => log::info!("atualização: {m}"),
                    Err(e) => log::warn!("atualização: {e}"),
                }
                std::thread::sleep(INTERVALO);
            }
        });
    }
}

/// Liga a verificação de atualização em segundo plano (Windows; nada nos outros sistemas).
pub fn iniciar(repo: &'static str, exe: &'static str) {
    #[cfg(target_os = "windows")]
    windows::iniciar(repo, exe);
    #[cfg(not(target_os = "windows"))]
    let _ = (repo, exe);
}
