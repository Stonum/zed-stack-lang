# Changelog

## Unreleased

The first Stack lang extension for Zed is in development.

- Added syntax highlighting, bracket matching, indentation, and outline support for Stack `.prg`, `.hdl`, and `.rpt` files.
- Added language support for Stack resources and dictionaries. The same Stack LSP session also handles standalone SQL files by default.
- Added 19 Stack snippets for common constructs and queries.
- Connected Stack language server features such as hover, navigation, diagnostics, and formatting. The development build can use a local server or download a verified binary. See the [server release manifest](server-release.json) for the version used by the downloader.
