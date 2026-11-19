# Stack for Zed — local development

Stack Tree-sitter grammar and local stdio LSP integration. This project is not ready for the extension catalog.

## Build and test

Prerequisites: Rust/Cargo with `wasm32-wasip2`, Node.js for evaluating `grammar.js` during generation, a C compiler for native corpus tests, and a local Stack language server. npm is not used.

```powershell
cargo install tree-sitter-cli --version =0.25.10 --locked --root .local/tree-sitter-cli
cargo run --locked -p stack-dev -- generate
cargo run --locked -p stack-dev -- test
cargo run --locked -p stack-dev -- queries
cargo build --locked --target wasm32-wasip2
cargo test --locked --workspace
cargo fmt --all -- --check
cargo clippy --locked --target wasm32-wasip2 -- -D warnings
cargo clippy --locked -p stack-dev -- -D warnings
cargo run --locked -p stack-dev -- smoke E:/projects/rust/stack-lang-server/target/debug/stack-lang-server.exe
./scripts/prepare-dev.ps1
```

The Rust development tool in `tools/` checks for CLI 0.25.10 in `.local/tree-sitter-cli/bin`, then PATH; it rejects other versions. Generation uses ABI 14 for compatibility. Generated C and headers are committed; downloaded CLI binaries and compiled artifacts are ignored. The extension and development tool share the root Cargo workspace and lockfile. Default workspace commands build only the extension; use `-p stack-dev` for native tooling.

## Install the local dev extension

Run **zed: install dev extension**, selecting the absolute `.local/dev-extension` directory printed by `prepare-dev.ps1`. Do not select the repository root. Zed's builder requires a Git checkout at `grammars/stack`; the managed grammar source there deliberately has no nested repository. The script copies the extension into an ignored staging directory, snapshots grammar sources into a separate local Git repository and writes a `file:///` repository URL with an exact commit into the staged manifest. It does not publish or change global settings. Re-run it after edits, then rebuild the dev extension in Zed. Zed may download the official wasi-sdk on first build.

The grammar can later be extracted into its own repository, or referenced from a published commit of this repository using `path = "grammars/stack"`. Catalog submission needs a reachable HTTPS repository and pinned full revision under `[grammars.stack]`; local file URLs must never be submitted.

## Local LSP configuration

Merge `examples/settings.json` into your test project's `.zed/settings.json` and adjust `binary.path`. Use a plain absolute executable path, without embedded quotes, even when it contains spaces. Arguments are separate array entries. `binary.env` overrides the inherited shell environment. When neither the server's settings nor its fallback below supplies `binary.path`, the extension uses `worktree.which("stack-lang-server")`; restart Zed after changing PATH. This lookup uses the worktree's PATH, before `binary.env` overrides are applied to the launched process. There is no downloader.

Defaults: `lens_enabled=false`, `ini_path=""`. Set `lsp.stack-lang-server.initialization_options.ini_path` when needed, then restart the language server. Empty string selects workspace discovery. Keep lenses disabled because Stack navigation commands require a client implementation.

## Grammar scope

`.prg`, `.hdl`, `.rpt` and uppercase equivalents share Stack syntax. Implemented: case-insensitive Russian/English keywords, functions, classes/methods, declarations, conditionals, loops, exceptions, switch, operator precedence, calls/members/subscripts, collections, numbers/dates/times, quoted and multiline strings, comments, and RPT Form Feed headers/sections. Queries provide highlights, bracket pairs, indentation and outline. Tree-sitter recovery preserves subsequent declarations after malformed statements; missing delimiters may produce ERROR/MISSING nodes while editing.

This first grammar is not a replacement for the LSP parser. Advanced annotations, destructuring, anonymous functions and string interpolation need further coverage. Embedded SQL stays string content. Resources, dictionaries and opt-in standalone SQL are described below; snippets and distribution are later stages.

## Resources and SQL

Stack Resource uses LSP ID `stack-resource`, tab size 3; Stack Dictionary uses `stack-dictionary`, tab size 4. Both use the MIT XML grammar pinned in extension.toml. Upstream highlights and license are retained. No XML-wide suffix is claimed.

Default suffixes cover rx, rx_api, rxz, rxi, rxe, rx_z in lower/uppercase, plus observed RX_b2b. Other real variants include rx_api_google_auth, RX_SURVEY, rx_zi_gis_dlg and RX_ZI_GIS_temp. Suffixes are literal, not globs. Merge [stack-project.json](examples/stack-project.json) into the Stack project's `.zed/settings.json` for arbitrary rx variants and case. This opt-in glob also matches future rx suffixes; narrow it if your project has unrelated rx files. Explicit tab sizes override user settings.

Install Zed's existing SQL extension (tested 1.1.8). The example enables `stack-sql-server` only in that project, with protocol ID `sql`. This separate opt-in registration launches its own process. Its binary settings inherit from `lsp.stack-lang-server.binary` per field:

- `path`: SQL path, then Stack path, then `worktree.which("stack-lang-server")`.
- `arguments`: SQL array, then Stack array, then no arguments. An explicit SQL `[]` clears inherited arguments.
- `env`: inherited shell environment, then Stack entries, then SQL entries, with later values winning per key. SQL `{}` retains inherited entries.

For example, `"stack-sql-server": {"binary": {"arguments": [], "env": {"RUST_LOG": "debug"}}}` under `lsp` keeps the Stack binary path while replacing arguments and overriding that environment key.

Configure SQL initialization options under `lsp.stack-sql-server.initialization_options`; defaults disable lenses and use workspace discovery. The example selects Stack as SQL's sole server; use `["stack-sql-server", "..."]` to retain others. Remove this entry and restart servers to opt out. Keep this setting project-local.

Zed 1.29.2 supports `opt_in_languages`, but a shared running server can still receive SQL documents, so SQL deliberately has a separate registration. No universal SQL language or injection highlighting is supplied.

Additional native query checks, without npm:

```powershell
git clone https://github.com/tree-sitter-grammars/tree-sitter-xml .local/tree-sitter-xml
git -C .local/tree-sitter-xml checkout 5000ae8f22d11fbe93939b05c1e37cf21117162d
cargo run --locked -p stack-dev -- xml-queries
```

The native smoke additionally opens deliberately unformatted resources, dictionaries and SQL and requires nonempty formatting edits for each. Every diagnostic error/clear check matches the document URI, version and expected diagnostic state, even when other documents have published the same version. Its IDs are test inputs; editor IDs require the separate Zed trace. Smoke artifacts live in `.local/lsp-smoke`. Closed-file watching, dynamic workspace roots and the extended editor acceptance matrix remain unverified; SQL injection highlighting is not provided.

The native `smoke` command copies the supplied server into `.local/lsp-smoke/server with spaces.exe`, uses two isolated workspace folders, and writes `trace.jsonl`, `results.json` and `stderr.txt` there. It checks hover, the completion response shape (an empty list is valid), symbols, semantic tokens, formatting, diagnostic error/clear and shutdown. It waits for Stack readiness and diagnostics with timeouts; it is not a Zed UI test.
