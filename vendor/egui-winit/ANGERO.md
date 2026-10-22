# egui-winit 0.36.2 — cópia com alteração do Office Angerô

Cópia do crate [`egui-winit`](https://github.com/emilk/egui/tree/main/crates/egui-winit) 0.36.2
(MIT OR Apache-2.0), usada pelo `[patch.crates-io]` do `Cargo.toml` da raiz.

O que mudou (`src/lib.rs`, só tem efeito no Windows): a "sombra de janela sem barra"
(`set_undecorated_shadow`) fica sempre desligada. Esse truque do winit desce a área da janela em
1 pixel, e com o DirectX 12 a imagem inteira era espremida em uma linha de pixel — letras e linhas
da grade desfocadas, principalmente em telas de 100 %. Sem ele a tela mostra exatamente o que o
programa desenha. As bordas para redimensionar a janela são desenhadas pelo próprio programa
(`crates/ui-egui/src/janela.rs`, `bordas`).
