# Stack for Zed — local LSP scaffold

Stage 2 only. This is not ready for the extension catalog. A real Stack Tree-sitter grammar is required in stage 3 before editor integration can be accepted.

## Build

Prerequisites: Rust/Cargo with `wasm32-wasip2` already installed, and a local Windows x64 Stack language server.

```powershell
cargo build --locked --target wasm32-wasip2
cargo fmt -- --check
cargo clippy --locked --target wasm32-wasip2 -- -D warnings
node scripts/stdio-smoke.cjs E:/projects/rust/stack-lang-server/target/debug/stack-lang-server.exe
```

Cargo.lock pins dependencies. The smoke test copies the supplied executable into `.local/stdio smoke/server with spaces.exe`, creates isolated fixture folders and writes JSON-RPC trace/results there. It never edits the original server or project sources.

## Local Zed configuration

Merge `examples/settings.json` into your test project's `.zed/settings.json` and adjust `binary.path`. Use a plain absolute executable path, without embedded quotes, even when it contains spaces. Arguments are separate array entries. `binary.env` overrides the inherited worktree shell environment. If `binary.path` is absent, the extension uses `worktree.which("stack-lang-server")`; restart Zed after changing PATH. There is no downloader.

Initialization defaults are `lens_enabled=false`, `ini_path=""`. Set `lsp.stack-lang-server.initialization_options.ini_path` to your Stack INI path when needed, then restart the language server. Empty string selects workspace discovery. Keep `lens_enabled=false`: Stack navigation commands require a client implementation. Other initialization options are preserved. Zed itself may merge user options after the extension hook, so the example explicitly disables lenses too.

Use **zed: install dev extension** and select this repository. There is intentionally no `languages/stack` grammar placeholder. Until stage 3, installing the scaffold does not establish that `.prg` files are recognized or that the LSP is activated. Do not assign a JavaScript grammar to Stack.
