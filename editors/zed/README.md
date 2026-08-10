# Sashimi for Zed

This directory contains the Zed language extension for Sashimi.

It provides:

- `.sashimi` language detection.
- Tree-sitter based syntax highlighting.
- The `sashimi lsp` language server.
- Compiler diagnostics on open/change.
- Completion for Sashimi keywords, core/prelude types and iterator methods, plus declarations in the current document.
- Hover documentation for core traits and iterator operations.
- Document symbols.

## Development

From the repository root, enter the Rust development environment:

```sh
nix develop
```

Then in Zed:

1. Open the Extensions view.
2. Choose **Install Dev Extension**.
3. Select `editors/zed` from this repository.
4. Open a `.sashimi` file.

The extension first looks for `sashimi` on `PATH`. When developing inside the Sashimi repository it falls back to `cargo run --quiet -- lsp`, so installing the compiler globally is not required.

## Highlighting grammar

The bootstrap extension currently reuses `tree-sitter-rust` because Sashimi deliberately shares Rust-like `fn`, `trait`, `impl`, `let`, receiver and expression syntax. `highlights.scm` adds the Sashimi-specific `class`, `new`, TypeScript-style primitive names, and core collection/iterator names.

A dedicated `tree-sitter-sashimi` grammar should replace this bootstrap grammar once the surface syntax stabilizes.
