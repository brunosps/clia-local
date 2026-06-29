# AdvPL / TLPP TextMate grammar — attribution

`advpl.tmLanguage.json` and `advpl-language-configuration.json` are vendored from
the **TOTVS Developer Studio for VS Code** project:

- Source: https://github.com/totvs/tds-vscode (`syntaxes/advpl_language.tmLanguage.json`,
  `advpl-language-configuration.json`)
- License: **Apache License 2.0** (full text in `./LICENSE`)
- Copyright © TOTVS S.A. and tds-vscode contributors

These files are used unmodified to provide AdvPL/TLPP (`.prw`, `.tlpp`, `.prg`, …)
syntax highlighting in the Monaco editor, bridged via `vscode-textmate` +
`vscode-oniguruma` (see `../advpl.ts`). Only syntax highlighting is ported; the
TOTVS language server (compile/debug/IntelliSense) is proprietary and not included.
