# epaint 0.36.2 — cópia com alterações do Office Angerô (Excel, Word, PowerPoint)

Cópia do crate [`epaint`](https://github.com/emilk/egui/tree/main/crates/epaint) 0.36.2
(MIT OR Apache-2.0, veja `LICENSE-MIT`), usada pelo `[patch.crates-io]` do `Cargo.toml` da raiz.

O que mudou (só no Windows):

- `src/text/dwrite.rs` (novo): cada letra é desenhada pelo DirectWrite, o motor de texto do Windows,
  no modo GDI clássico (encaixe forte nos pixels, como o Office em telas de 100 %), em vez do
  rasterizador do egui. Se o DirectWrite não abrir a fonte, vale o desenho original.
- `src/text/font.rs`: `FontFace` guarda o índice da face; `allocate_glyph` tenta o DirectWrite
  primeiro; `hinted_advance_px` devolve a largura da letra ajustada aos pixels inteiros do GDI.
- `src/text/text_layout.rs`: o posicionamento usa essa largura ajustada (senão as letras encaixadas
  ficam com buracos entre si: "ST ELLA").
- `Cargo.toml`: dependência `windows` (DirectWrite) só no Windows.

`ANGERO_TEXTO=egui` volta ao desenho original (para comparar); `natural` e `simetrico` usam outros
modos do DirectWrite.
