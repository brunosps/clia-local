# codex — clia-local: versão exibida com fonte única (bug: UI mostra 0.2.0 com app 0.2.2)

Worktree `/home/bruno/code/clia-local-version-fix` (branch `fix/version-single-source`). O card de
status mostra VERSION 0.2.0 porque `src/App.tsx:457` lê `packageInfo.version` do `package.json`
raiz (0.2.0), enquanto `src-tauri/tauri.conf.json` e `src-tauri/Cargo.toml` já estão em 0.2.2 —
três fontes dessincronizadas. Commits atômicos, sem push.

## Fix

1. **Fonte única = versão do app Tauri**: no front, obter a versão via `getVersion()` de
   `@tauri-apps/api/app` (async — carregue no boot/estado e passe pro card; siga o padrão de
   invocação/estado já usado no App.tsx). Fallback: `packageInfo.version` se a API falhar
   (ex.: contexto de dev browser sem Tauri, se aplicável ao projeto).
2. **Sincronizar `package.json` para 0.2.2** mesmo assim (higiene; é fallback e metadado npm).
3. Se o repo tiver script/checagem de release, adicione um teste barato que compare
   `package.json.version` == `tauri.conf.json.version` (falha de CI se dessincronizar de novo);
   se não houver harness pra isso em vitest, faça um teste vitest simples lendo os dois JSONs.

## Fence e gate

Worktree only; sem push; sem tocar `src-tauri/binaries/`. Gate: `corepack pnpm exec tsc --noEmit` ·
`corepack pnpm test` · `corepack pnpm exec vite build` · `cargo fmt --check`/`clippy -D warnings`/
`cargo test` no src-tauri SE tocar Rust (provavelmente não precisa). Auto-nota; repita enquanto <9.
Relatório: STATUS · fix→teste · VERIFICATION · auto-nota.
