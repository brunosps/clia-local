**STATUS**
Rebase concluído em `fix/compose-passthrough` sobre `main` 0.2.4. Worktree limpo, sem push.

Commit final local: `e077067 chore: finalize clia-local 0.2.5 rebase`.

**Conflitos Resolvidos**
- `src-tauri/src/deploy_package.rs`: combinei `dismissed_findings` do 0.2.4 com `compose_mode`/`compose_decision.warnings` do passthrough no manifest.
- `src-tauri/src/deploy_package.rs`: mantive toda a lógica de dismiss/restore/herança/auditoria e acrescentei `append_validation_warning`.
- `src-tauri/src/deploy_package.rs`: mantive o `.env.example` gerado fora do scan; compose root continua escaneado e `.env.example` de repo continua coberto via source copy.
- `src-tauri/src/deploy_env.rs`: ajustei initializer de teste para o campo novo `dismissed_findings_json`.
- `package.json`, `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml`: versão em `0.2.5`; `src-tauri/Cargo.lock` atualizado para o pacote raiz.

`DeployPackagesPanel.tsx`, `deploy.ts` e `styles.css` auto-mergearam sem conflito manual; o gate frontend validou o resultado.

**VERIFICATION**
- `cargo fmt --check --manifest-path src-tauri/Cargo.toml` passou
- `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings` passou
- `cargo test --manifest-path src-tauri/Cargo.toml` passou: 249 testes
- `corepack pnpm exec tsc --noEmit` passou
- `corepack pnpm test` passou: 230 testes
- `corepack pnpm exec vite build` passou, com o warning normal de chunks grandes do Vite
- `main` confirmado como ancestral de `HEAD`
- Sem marcadores de conflito restantes

**Auto-nota**
9.4/10. O rebase preservou as duas semânticas e o gate completo ficou verde; único ajuste encontrado no caminho foi um initializer de teste sem o campo novo, corrigido antes do commit final.