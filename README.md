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
```

The Rust development tool in `tools/` checks for CLI 0.25.10 in `.local/tree-sitter-cli/bin`, then PATH; it rejects other versions. Generation uses ABI 14 for compatibility. Generated C and headers are committed; downloaded CLI binaries and compiled artifacts are ignored. The extension and development tool share the root Cargo workspace and lockfile. Default workspace commands build only the extension; use `-p stack-dev` for native tooling.

## Install the local dev extension

1. On Windows, run this from the repository root. It writes the local `extension.toml` from `extension.toml.in` and pins a Git snapshot of the Stack grammar in `.local/`. The execution policy applies to this PowerShell process only.

   ```powershell
   powershell.exe -NoProfile -ExecutionPolicy RemoteSigned -File .\scripts\prepare-dev.ps1
   ```

2. Run **zed: install dev extension** and select the **repository root**. If an earlier dev install points at `.local/dev-extension`, install again with the root selected.

Zed loads the extension code and language queries directly from the project. It checks out the Stack and XML grammars under `grammars/` and builds them with the official WASI SDK, which it may download on first use. The editable Stack grammar lives in `tree-sitter/stack/`. After changing it, regenerate the parser and run `prepare-dev.ps1` again. Also rerun the script after editing `extension.toml.in` or moving the checkout. Use **Rebuild** in Zed to pick up changes.

For catalog submission, use a published `extension.toml` with a reachable HTTPS URL and pinned Stack grammar revision. A published commit of this repository can supply the grammar with `path = "tree-sitter/stack"`.

## Local LSP configuration

To use a local build, merge `examples/settings.json` into your test project's `.zed/settings.json` and adjust `binary.path`. Use a plain absolute executable path, without embedded quotes, even when it contains spaces. Arguments are separate array entries. `binary.env` overrides the inherited shell environment. Without a configured path, the extension looks for `stack-lang-server` in the worktree's PATH before applying `binary.env`. Restart Zed after changing PATH.

If neither source provides a binary, the extension downloads the Stack language server release pinned in [server-release.json](server-release.json). The manifest lists Windows x64 and Linux x64 assets. Other platforms require a local binary. The extension verifies SHA-256 before installation and reuses the verified copy in its work directory without network access. Remove the local binary path and ensure the server is absent from the worktree's PATH to test this fallback.

Set `lsp.stack-lang-server.initialization_options.ini_path` when needed, then restart the language server. The default empty string selects workspace discovery.

The extension enables CodeLens support on the server by default. Use Zed's `code_lens` setting to control it in the editor. `"off"` (the default) skips CodeLens requests, `"on"` displays lenses above the code, and `"menu"` puts them in the code actions menu. Navigation requires a server build that supports `editor.action.goToLocations`. Use a local build until the extension pins a release with this support.

```json
{
  "code_lens": "on"
}
```

If you previously set `lsp.stack-lang-server.initialization_options.lens_enabled` to `false`, remove that override and restart the language server so Zed can request lenses when enabled.

Stack LSP discovers its workspace folders and registers file watchers at startup. With workspace discovery, Zed can start a separate server for each worktree. To navigate between those roots, point each worktree's `ini_path` at the same `stack.ini` and list the source directories from all roots under `[AppPath]`. After adding or removing folders in VS Code, reload the window so the server discovers the updated workspace.

## Versioning

When adopting a server release, update the tag and both platform assets and hashes together in `server-release.json`. The downloader embeds this file at build time, and Windows CI reads the same file for the smoke test. Record the changes users will see in `CHANGELOG.md` and link to the server release notes.

When releasing the extension, move the `Unreleased` entries under the new extension version and keep the version in `extension.toml.in`, `Cargo.toml`, and `Cargo.lock` in sync. An extension-only change can be released with the same server pin.

## Grammar scope

`.prg`, `.hdl`, `.rpt` and uppercase equivalents share Stack syntax. Implemented: case-insensitive Russian/English keywords, functions, classes/methods, declarations, conditionals, loops, exceptions, switch, operator precedence, calls/members/subscripts, collections, numbers/dates/times, quoted and multiline strings, comments, and RPT Form Feed headers/sections. Queries provide highlights, bracket pairs, indentation and outline. Tree-sitter recovery preserves subsequent declarations after malformed statements; missing delimiters may produce ERROR/MISSING nodes while editing.

This first grammar is not a replacement for the LSP parser. Advanced annotations, destructuring, anonymous functions and string interpolation need further coverage. Embedded SQL stays string content. Resources, dictionaries, standalone SQL and snippets are described below; distribution is a later stage.

## Resources and SQL

Stack Resource uses LSP ID `stack-resource`, tab size 3; Stack Dictionary uses `stack-dictionary`, tab size 4. Both use the MIT XML grammar pinned in `extension.toml.in`. Upstream highlights and license are retained. No XML-wide suffix is claimed.

Default suffixes cover rx, rx_api, rxz, rxi, rxe, rx_z in lower/uppercase, plus observed RX_b2b. Other real variants include rx_api_google_auth, RX_SURVEY, rx_zi_gis_dlg and RX_ZI_GIS_temp. Suffixes are literal, not globs. Merge [stack-project.json](examples/stack-project.json) into the Stack project's `.zed/settings.json` for arbitrary rx variants and case. This opt-in glob also matches future rx suffixes; narrow it if your project has unrelated rx files. Explicit tab sizes override user settings.

Install Zed's SQL extension (tested with version 1.1.8) to register the `SQL` language. The Stack extension enables `stack-lang-server` for `.sql` files by default. Zed sends them with LSP language ID `sql` to the same server session as Stack sources, resources, and dictionaries in the worktree. The server understands Stack's SQL syntax and uses the shared `lsp.stack-lang-server` settings.

The project example selects `stack-lang-server` as the sole SQL server for diagnostics and formatting. If you applied an earlier version of the example, update the SQL server list to `["stack-lang-server"]` and move any SQL-specific overrides you still need into `lsp.stack-lang-server`, where they apply to the shared session.

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
| CodeLens navigation | Stack sends `editor.action.goToLocations` with the source URI, source position, and target locations. Zed's [client command handler](https://github.com/zed-industries/zed/blob/2de9e435a5d98657f35c260b5f4dd0979df659e6/crates/editor/src/code_lens.rs) uses those locations to navigate. |
| CodeLens refresh | Zed advertises `workspace.codeLens.refreshSupport` and [handles refresh requests](https://github.com/zed-industries/zed/blob/2de9e435a5d98657f35c260b5f4dd0979df659e6/crates/project/src/lsp_store.rs). Stack requests startup refresh when the client advertises this capability and lenses are enabled. |
| SQL transformations | `stack.togglePostgreSQL`, `stack.fieldsToPostgreSQL` and `stack.toggleMsSQL` depend on VS Code selections and editor edits. Zed's [public Extension trait](https://github.com/zed-industries/zed/blob/2de9e435a5d98657f35c260b5f4dd0979df659e6/crates/extension_api/src/extension_api.rs) exposes neither arbitrary editor commands nor selection edits. A separate common-LSP task should offer range-based CodeAction + WorkspaceEdit, with explicit namespace/direction/dollar options and documented treatment of multiple selections. Zed supports code actions and workspace edits; the current Stack server does not provide these transformations. No equivalent commands are installed here. |
| Indexing status | `custom/statusBar` is a VS Code-specific notification. Zed's extension installation status reports installation, not indexing. The supported future route is standard `window/workDoneProgress/create` and `$/progress` begin/report/end, gated on `window.workDoneProgress`; Zed advertises and handles these. Add this in a separate common-LSP task, preserve existing clients as needed, and test completion/error/cancellation cleanup. No live progress UI test or custom-notification adapter is claimed. |

The [official snippet extension guide](https://zed.dev/docs/extensions/snippets) describes manifest paths and filename scopes; the pinned [registry](https://github.com/zed-industries/zed/blob/2de9e435a5d98657f35c260b5f4dd0979df659e6/crates/snippet_provider/src/registry.rs), [loader](https://github.com/zed-industries/zed/blob/2de9e435a5d98657f35c260b5f4dd0979df659e6/crates/snippet_provider/src/lib.rs) and [parser](https://github.com/zed-industries/zed/blob/2de9e435a5d98657f35c260b5f4dd0979df659e6/crates/snippet/src/snippet.rs) were also checked. No neighboring repository changes, watcher work, downloader, grammar rewrite or publication are part of this stage.
