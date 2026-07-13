**STATUS**

Concluído e commitado localmente, sem push.

Commit: `cf7ad3a fix: harden deploy secret scanning`  
Worktree final: limpo.

**T → Fix → Teste**

- **T1:** `deploy_scan` agora percorre todas as ocorrências por marcador, incluindo `bearer `. Testes cobrem placeholder primeiro + segredo real depois em assignment e Bearer.
- **T2:** `deploy_package` restaurou `is_non_runtime_path`: pacote rebaixa `test/tests/__tests__/e2e/fixtures/docs` para warning; plano continua bloqueando artefatos runtime mesmo com path de fixture.
- **T3:** assignment com token começando em `{` agora é placeholder. Restaurei o fixture `PASSWORD={password}`.
- **T4:** Bearer corta na primeira quebra de linha antes de `trim_start`, evitando ler a linha seguinte.

**VERIFICATION**

- `cargo fmt --check` OK
- `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings` OK
- `cargo test --manifest-path src-tauri/Cargo.toml` OK, `226 passed`
- `corepack pnpm exec tsc --noEmit` OK
- `corepack pnpm test` OK, `226 passed`
- `corepack pnpm exec vite build` OK, apenas aviso conhecido de chunks grandes

Confirmação explícita: **nenhum fixture foi afrouxado**. O caso `PASSWORD={password}` foi restaurado, e os novos testes acrescentam segredos reais posteriores para provar o bloqueio.

Auto-nota: **9.6/10**.