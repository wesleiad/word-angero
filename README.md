# Word - Angerô

Programa do **Office Angerô**, o pacote de escritório do **Grupo Angerô**.

Editor de textos: documentos, ofícios, relatórios e mala direta; abre e salva .docx.

- Ajuda: https://github.com/wesleiad/Office-Angero/blob/main/docs/ajuda/word.md
- Office Angerô (ícones, instaladores, ajuda): https://github.com/wesleiad/Office-Angero
- Versões compiladas: aba **Releases** deste repositório (o programa instalado se atualiza sozinho a partir delas).

## Compilar

Windows, com Rust (MSVC) e Visual Studio Build Tools (C++):

```
cargo build --release --locked -p wordcraft -p wordcraft-cli
```

Gera `target/release/word-angero.exe` e `target/release/word-angero-cli.exe`.
A cada push no ramo `angero`, o GitHub Actions compila e publica uma versão nova em Releases.

## Código-base e licença

Desenvolvido por Weslei Domingues (TI) para o Grupo Angerô, a partir do código aberto do
**WordCraft** (projeto ArtCraft, https://github.com/storytold/wordcraft), usado como base.
Licença: MIT OU Apache-2.0, à escolha — veja `LICENSE-MIT`, `LICENSE-APACHE` e `NOTICE`.
