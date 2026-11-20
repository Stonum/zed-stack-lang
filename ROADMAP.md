# Stack for Zed — remaining work

Catalog submission is on hold until the language-server issues are resolved and verified. The extension can be developed and tested locally in the meantime.

## Local extension work

- Add a versioned download fallback after configured binary paths and PATH. Use an existing server release for development and replace the pin once a release with the required LSP fixes passes acceptance.
- Verify the extension from a clean checkout. Check first download, offline reuse, launch overrides, and the existing grammar, snippets, resource and SQL workflows. Verify a concrete completion result in the editor. An empty but valid completion response only verifies the protocol.
- Retest the Windows x64 and Linux x64 binaries separately before claiming support for both platforms.

## Common language server

Implement server changes in `stack-lang-server`. Keep extension-specific changes in this repository.

- **Closed-file index (blocking).** Stage 6a observed outdated HDL symbols 0.23 seconds after an external edit/create/delete. Zed advertised dynamic watcher registration, but the trace contained neither registration nor file-change events before the session ended. There was no later symbol request and no RX or restart control. Repeat with an identified server build, query after 2, 10 and 30 seconds, and compare with a restart. Check that HDL symbols and RX references update while the affected files stay closed. If the failure persists, fix subscription and index maintenance in the server and test Zed and VS Code with the same build, including the effect of existing VS Code file watchers. See the local `docs/stage6a.md` for the fixture and trace details.
- **Workspace roots.** Verify initial roots and root changes separately from file events. Document the supported behavior before claiming dynamic root support.
- **Client parity.** Keep CodeLens disabled until its location commands work in Zed and VS Code. Track range-based SQL transformations and standard LSP work progress as separate server work. Their implementation scope and order are independent of local extension tooling.

## Deferred catalog work

After server acceptance, pin the verified release, use a public HTTPS revision for the Stack grammar, add the extension license at the root, confirm the catalog ID, and manually test the exact submission commit. Submit to the catalog in a separate step.
