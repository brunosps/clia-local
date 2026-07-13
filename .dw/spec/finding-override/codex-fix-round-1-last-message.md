**STATUS**  
Aprovado no worktree local. Commit criado, sem push:

`f3f0ee5 fix: stabilize deploy review dismissals`

**W → Fix → Teste**  
- **W1:** findings agora usam `path` relativo ao pacote + `marker` + `line_sha256` da linha normalizada, com `line_number`; scanner reporta todas as ocorrências. Testes reais com `create_package`: v1 dismiss, v2 herda, v3 com token adicional bloqueia só o novo; mudança de linha/path não herda.
- **W2:** restore grava tombstone via audit e herança respeita restore; `inherited_from_label` aponta para a origem real do dismiss. Teste: dismiss v1 → herda até v3 → restore v3 → v4 bloqueia.
- **W3:** `review_audit_json` append-only em `deploy_versions`; dismiss duplicado appenda, restore não apaga histórico. UI mostra histórico no detalhe do finding.
- **W4:** dismiss só em review pendente; restore de finding não-dismissado agora erra; update de JSON roda em transação SQLite `IMMEDIATE`.
- **W5:** `appVersion.test.ts` agora compara `package.json`, `tauri.conf.json` e `src-tauri/Cargo.toml`.

**VERIFICATION**  
- `cargo fmt --manifest-path src-tauri/Cargo.toml --check`
- `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`
- `cargo test --manifest-path src-tauri/Cargo.toml` → 233 ok
- `corepack pnpm typecheck`
- `corepack pnpm test` → 228 ok
- `corepack pnpm build:web` → ok, apenas warning padrão de chunk grande do Vite

Worktree final limpo.

**Auto-nota:** 9.6/10.