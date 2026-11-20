# Stack for Zed — remaining work

Catalog submission is on hold until the language-server issues are resolved and verified. The extension can be developed and tested locally in the meantime.

## Local extension work

- A versioned fallback follows configured binary paths and PATH. The downloader and Windows CI share the development pin and checksums in [server-release.json](server-release.json). Update them once a release with the required LSP fixes passes acceptance.
- On Windows, a clean checkout built the WASM extension, passed Rust and Tree-sitter checks, prepared the dev extension and passed the stdio smoke using the verified release binary. Automated tests cover rejecting bad downloads and reusing a valid cache without a network call.
- Editor acceptance remains open for a first download and an offline restart in Zed, the Linux x64 runtime, and a concrete completion result. The stdio fixture received a valid but empty completion response. Repeat the grammar, snippets, resource and SQL checks with the server candidate before catalog submission.
- The Windows CI workflow runs build, lint, tests and dev extension preparation. A hosted run becomes available once the repository is on GitHub.

## Common language server

Implement server changes in `stack-lang-server`. Keep extension-specific changes in this repository.

- **Closed-file index.** The server now registers watched files. Local Windows tests verified that Zed and VS Code update closed HDL symbols and RX references after external changes, including in directories listed in `stack.ini`. The VS Code client uses the server subscription. Update the pinned server release before publishing either extension with this behavior. See the local `docs/stage6a.md` for the original reproduction.
- **Workspace roots.** The server uses the roots available at startup. In the Windows test, Zed started separate server sessions for Stack files in two worktrees. A shared `stack.ini` let one session index both source roots. VS Code supplied both folders at startup. Replacing the second folder took effect on a fresh server launch. The server advertises support for initial workspace folders and does not subscribe to folder change notifications.
- **CodeLens.** The server sends one location command for both editors. Zed can opt in with a compatible local server build. Update both extensions' pinned server release before publishing changes to CodeLens.
- **Client parity.** Range-based SQL transformations and standard LSP work progress remain separate server tasks.

## Deferred catalog work

After server acceptance, pin the verified release, use a public HTTPS revision for the Stack grammar, add the extension license at the root, confirm the catalog ID, and manually test the exact submission commit. Submit to the catalog in a separate step.
