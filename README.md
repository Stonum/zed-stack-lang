# Stack for Zed — local development

Stack Tree-sitter grammar and stdio LSP integration. See the [changelog](CHANGELOG.md) for changes and the [roadmap](ROADMAP.md) for the remaining work.

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

To use a local build, merge `examples/settings.json` into your test project's `.zed/settings.json` and adjust `binary.path`. Use a plain absolute executable path, without embedded quotes, even when it contains spaces. Arguments are separate array entries. `binary.env` overrides the inherited shell environment. Without a configured path, the extension looks for `stack-lang-server` in the worktree's PATH before applying `binary.env`. Restart Zed after changing PATH.

If neither source provides a binary, the extension downloads the Stack language server release pinned in [server-release.json](server-release.json). The manifest lists Windows x64 and Linux x64 assets. Other platforms require a local binary. The extension verifies SHA-256 before installation and reuses the verified copy in its work directory without network access. Both server registrations share the download cache. Remove the local binary path and ensure the server is absent from the worktree's PATH to test this fallback.

Defaults: `lens_enabled=false`, `ini_path=""`. Set `lsp.stack-lang-server.initialization_options.ini_path` when needed, then restart the language server. Empty string selects workspace discovery. Keep lenses disabled because Stack navigation commands require a client implementation.

Stack LSP discovers its workspace folders and registers file watchers at startup. With workspace discovery, Zed can start a separate server for each worktree. To navigate between those roots, point each worktree's `ini_path` at the same `stack.ini` and list the source directories from all roots under `[AppPath]`. After adding or removing folders in VS Code, reload the window so the server discovers the updated workspace.

## Versioning

When adopting a server release, update the tag and both platform assets and hashes together in `server-release.json`. The downloader embeds this file at build time, and Windows CI reads the same file for the smoke test. Record the changes users will see in `CHANGELOG.md` and link to the server release notes.

When releasing the extension, move the `Unreleased` entries under the new extension version and keep the version in `extension.toml`, `Cargo.toml`, and `Cargo.lock` in sync. An extension-only change can be released with the same server pin.

## Grammar scope

`.prg`, `.hdl`, `.rpt` and uppercase equivalents share Stack syntax. Implemented: case-insensitive Russian/English keywords, functions, classes/methods, declarations, conditionals, loops, exceptions, switch, operator precedence, calls/members/subscripts, collections, numbers/dates/times, quoted and multiline strings, comments, and RPT Form Feed headers/sections. Queries provide highlights, bracket pairs, indentation and outline. Tree-sitter recovery preserves subsequent declarations after malformed statements; missing delimiters may produce ERROR/MISSING nodes while editing.

This first grammar is not a replacement for the LSP parser. Advanced annotations, destructuring, anonymous functions and string interpolation need further coverage. Embedded SQL stays string content. Resources, dictionaries, opt-in standalone SQL and snippets are described below; distribution is a later stage.

## Resources and SQL

Stack Resource uses LSP ID `stack-resource`, tab size 3; Stack Dictionary uses `stack-dictionary`, tab size 4. Both use the MIT XML grammar pinned in extension.toml. Upstream highlights and license are retained. No XML-wide suffix is claimed.

Default suffixes cover rx, rx_api, rxz, rxi, rxe, rx_z in lower/uppercase, plus observed RX_b2b. Other real variants include rx_api_google_auth, RX_SURVEY, rx_zi_gis_dlg and RX_ZI_GIS_temp. Suffixes are literal, not globs. Merge [stack-project.json](examples/stack-project.json) into the Stack project's `.zed/settings.json` for arbitrary rx variants and case. This opt-in glob also matches future rx suffixes; narrow it if your project has unrelated rx files. Explicit tab sizes override user settings.

Install Zed's existing SQL extension (tested 1.1.8). The example enables `stack-sql-server` only in that project, with protocol ID `sql`. This separate opt-in registration launches its own process. Its binary settings inherit from `lsp.stack-lang-server.binary` per field:

- `path`: SQL path, then Stack path, then worktree PATH, then the shared download fallback.
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

The native smoke additionally opens deliberately unformatted resources, dictionaries and SQL and requires nonempty formatting edits for each. Every diagnostic error/clear check matches the document URI, version and expected diagnostic state, even when other documents have published the same version. Its IDs are test inputs; editor IDs require the separate Zed trace. Smoke artifacts live in `.local/lsp-smoke`. See the [roadmap](ROADMAP.md) for editor acceptance results and remaining checks. SQL injection highlighting is not provided.

The native `smoke` command copies the supplied server into `.local/lsp-smoke/server with spaces.exe`, uses two isolated workspace folders, and writes `trace.jsonl`, `results.json` and `stderr.txt` there. It checks hover, the completion response shape (an empty list is valid), symbols, semantic tokens, formatting, diagnostic error/clear and shutdown. It waits for Stack readiness and diagnostics with timeouts; it is not a Zed UI test.

## Snippets

The manifest registers `snippets/stack.json`: Zed uses the lowercase language name `stack` for the scope, while the language configuration and server registration use `Stack`. No global `snippets.json` is provided. The dev preparation script also copies the snippets directory.

The 19 Stack snippets retain the VS Code extension's Russian prefixes, keywords, default field values and expanded code, including whitespace. Names and descriptions are English. Type a prefix such as `Функция`, `ВыборПо`, `буф` or `выполнить`, select its completion and press Enter; Tab visits editable fields and then the final cursor position. Duplicate prefixes (`Если`, `еслислед`) retain both alternatives. The switch's unsupported named variables are now numbered placeholders; repeated editable `$0` fields use linked positive numbers followed by a final `$0`. Three source snippets scoped to SQL (`try`, `sql.update`, `sql.select`) are intentionally excluded from the Stack scope; this extension does not install SQL snippets globally.

Verified in Zed 1.29.2 through the staged dev extension in `.local/editor-test`: Function completion/insertion and name → parameters → body; Switch completion/insertion and all six cursor stops; Execute query completion/insertion, synchronized replacement of both query names, parameter and final stops. English labels/descriptions and nested indentation were visible. A Plain Text buffer with `Функция` showed no Stack completion, including after Ctrl+Space. This is a representative UI check, not an insertion test of all 19 snippets. Separately, all 19 bodies passed that Zed revision's official snippet parser in an isolated local Cargo harness, with exact expanded-text comparison against the source defaults (including tabs).

## Client feature limits (Zed 1.29.2)

The following assessment uses official Zed revision `2de9e435a5d98657f35c260b5f4dd0979df659e6`, matching the tested editor. Source compatibility is distinct from a live feature test.

| Feature | Confirmed support and next step |
| --- | --- |
| CodeLens navigation | Zed's [client command handler](https://github.com/zed-industries/zed/blob/2de9e435a5d98657f35c260b5f4dd0979df659e6/crates/editor/src/code_lens.rs) handles `editor.action.showReferences`, `editor.action.goToLocations` and `editor.action.peekLocations`. It does not recognize `stack.movetoLine` or `stack.goToLocations`. Unknown commands fall through to the code-action execution path, which skips commands absent from server executeCommand capabilities. Stack advertises only `dummy.do_something` and implements no corresponding Stack navigation handler. The public extension API cannot register an editor-command alias. Keep lenses disabled (the extension forces `lens_enabled=false`). A separate server task could emit the built-in location command with URI, position and Location[] arguments; convert a line-only target to a Location in the current document. Test single/multiple destinations and empty results in both clients before enabling lenses. |
| CodeLens refresh | Zed advertises `workspace.codeLens.refreshSupport` and [handles refresh requests](https://github.com/zed-industries/zed/blob/2de9e435a5d98657f35c260b5f4dd0979df659e6/crates/project/src/lsp_store.rs). Stack already gates startup refresh on that capability and enabled lenses. The existing editor initialize trace confirms the capability and disabled lenses; no live lens rendering, click or refresh acceptance test was performed. Refresh support alone does not make Stack navigation commands work. |
| SQL transformations | `stack.togglePostgreSQL`, `stack.fieldsToPostgreSQL` and `stack.toggleMsSQL` depend on VS Code selections and editor edits. Zed's [public Extension trait](https://github.com/zed-industries/zed/blob/2de9e435a5d98657f35c260b5f4dd0979df659e6/crates/extension_api/src/extension_api.rs) exposes neither arbitrary editor commands nor selection edits. A separate common-LSP task should offer range-based CodeAction + WorkspaceEdit, with explicit namespace/direction/dollar options and documented treatment of multiple selections. Zed supports code actions and workspace edits; the current Stack server does not provide these transformations. No equivalent commands are installed here. |
| Indexing status | `custom/statusBar` is a VS Code-specific notification. Zed's extension installation status reports installation, not indexing. The supported future route is standard `window/workDoneProgress/create` and `$/progress` begin/report/end, gated on `window.workDoneProgress`; Zed advertises and handles these. Add this in a separate common-LSP task, preserve existing clients as needed, and test completion/error/cancellation cleanup. No live progress UI test or custom-notification adapter is claimed. |

The [official snippet extension guide](https://zed.dev/docs/extensions/snippets) describes manifest paths and filename scopes; the pinned [registry](https://github.com/zed-industries/zed/blob/2de9e435a5d98657f35c260b5f4dd0979df659e6/crates/snippet_provider/src/registry.rs), [loader](https://github.com/zed-industries/zed/blob/2de9e435a5d98657f35c260b5f4dd0979df659e6/crates/snippet_provider/src/lib.rs) and [parser](https://github.com/zed-industries/zed/blob/2de9e435a5d98657f35c260b5f4dd0979df659e6/crates/snippet/src/snippet.rs) were also checked. No neighboring repository changes, watcher work, downloader, grammar rewrite or publication are part of this stage.
