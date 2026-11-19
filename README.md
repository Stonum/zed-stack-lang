# Stack for Zed — local development

Stack Tree-sitter grammar and local stdio LSP integration. This project is not ready for the extension catalog.

## Build and test

Prerequisites: Node/npm, Rust/Cargo with `wasm32-wasip2`, a C compiler for native corpus tests, and a local Stack language server.

```powershell
npm ci
npm run generate
npm test
cargo build --locked --target wasm32-wasip2
cargo fmt -- --check
cargo clippy --locked --target wasm32-wasip2 -- -D warnings
node scripts/stdio-smoke.cjs E:/projects/rust/stack-lang-server/target/debug/stack-lang-server.exe
./scripts/prepare-dev.ps1
```

CLI 0.25.10 is pinned by package-lock.json; generation uses ABI 14 for compatibility. Generated C and headers are committed; downloaded tools and compiled binaries are ignored.

## Install the local dev extension

Run **zed: install dev extension**, selecting the absolute `.local/dev-extension` directory printed by `prepare-dev.ps1`. Do not select the repository root. Zed's builder requires a Git checkout at `grammars/stack`; the managed grammar source there deliberately has no nested repository. The script copies the extension into an ignored staging directory, snapshots grammar sources into a separate local Git repository and writes a `file:///` repository URL with an exact commit into the staged manifest. It does not publish or change global settings. Re-run it after edits, then rebuild the dev extension in Zed. Zed may download the official wasi-sdk on first build.

The grammar can later be extracted into its own repository, or referenced from a published commit of this repository using `path = "grammars/stack"`. Catalog submission needs a reachable HTTPS repository and pinned full revision under `[grammars.stack]`; local file URLs must never be submitted.

## Local LSP configuration

Merge `examples/settings.json` into your test project's `.zed/settings.json` and adjust `binary.path`. Use a plain absolute executable path, without embedded quotes, even when it contains spaces. Arguments are separate array entries. `binary.env` overrides the inherited shell environment. Without `binary.path`, the extension uses `worktree.which("stack-lang-server")`; restart Zed after changing PATH. There is no downloader.

Defaults: `lens_enabled=false`, `ini_path=""`. Set `lsp.stack-lang-server.initialization_options.ini_path` when needed, then restart the language server. Empty string selects workspace discovery. Keep lenses disabled because Stack navigation commands require a client implementation.

## Grammar scope

`.prg`, `.hdl`, `.rpt` and uppercase equivalents share Stack syntax. Implemented: case-insensitive Russian/English keywords, functions, classes/methods, declarations, conditionals, loops, exceptions, switch, operator precedence, calls/members/subscripts, collections, numbers/dates/times, quoted and multiline strings, comments, and RPT Form Feed headers/sections. Queries provide highlights, bracket pairs, indentation and outline. Tree-sitter recovery preserves subsequent declarations after malformed statements; missing delimiters may produce ERROR/MISSING nodes while editing.

This first grammar is not a replacement for the LSP parser. Advanced annotations, destructuring, anonymous functions and string interpolation need further coverage. SQL stays string content; resources, dictionaries, standalone SQL, snippets and distribution are later stages.

Run `npm run test:queries` to validate Zed queries against the parser.
