# Stack for Zed — remaining work

Catalog submission is on hold until the language-server issues are resolved and verified. The extension can be developed and tested locally in the meantime.

## Local extension work

- A versioned fallback follows configured binary paths and PATH. The development pin is v0.8.3. Replace the pin and its checksums once a release with the required LSP fixes passes acceptance.
- On Windows, a clean checkout built the WASM extension, passed Rust and Tree-sitter checks, prepared the dev extension and passed the stdio smoke using the verified release binary. Automated tests cover rejecting bad downloads and reusing a valid cache without a network call.
- Editor acceptance remains open for a first download and an offline restart in Zed, the Linux x64 runtime, and a concrete completion result. The stdio fixture received a valid but empty completion response. Repeat the grammar, snippets, resource and SQL checks with the server candidate before catalog submission.
- The Windows CI workflow runs build, lint, tests and dev extension preparation. A hosted run becomes available once the repository is on GitHub.

## Common language server

Implement server changes in `stack-lang-server`. Keep extension-specific changes in this repository.

- **Closed-file index (blocking).** Stage 6a observed outdated HDL symbols 0.23 seconds after an external edit/create/delete. Zed advertised dynamic watcher registration, but the trace contained neither registration nor file-change events before the session ended. There was no later symbol request and no RX or restart control. Repeat with an identified server build, query after 2, 10 and 30 seconds, and compare with a restart. Check that HDL symbols and RX references update while the affected files stay closed. If the failure persists, fix subscription and index maintenance in the server and test Zed and VS Code with the same build, including the effect of existing VS Code file watchers. See the local `docs/stage6a.md` for the fixture and trace details.
- **Workspace roots.** Verify initial roots and root changes separately from file events. Document the supported behavior before claiming dynamic root support.
- **Client parity.** Keep CodeLens disabled until its location commands work in Zed and VS Code. Track range-based SQL transformations and standard LSP work progress as separate server work. Their implementation scope and order are independent of local extension tooling.

## Deferred catalog work

After server acceptance, pin the verified release, use a public HTTPS revision for the Stack grammar, add the extension license at the root, confirm the catalog ID, and manually test the exact submission commit. Submit to the catalog in a separate step.
