**STATUS**

Concluído e commitado localmente, sem push.

Commit: `8807f6d fix: make deploy review scanner placeholder-aware`

**S → Fix → Teste**

- **S1/S2:** extraí o scanner para `src-tauri/src/deploy_scan.rs` e reapliquei em plano + pacote. Assignments placeholder passam; valores reais bloqueiam. `Bearer {}`, `Bearer $VAR`, `Bearer "` e `%s` passam; token literal longo bloqueia.
- **S3:** `DATABASE_URL`/`SMTP_URL`/`REDIS_URL` detectados viram opcionais (`#KEY=`) quando há `artifacts.compose`; sem compose próprio continuam obrigatórios.
- **S4:** `SecretFinding` do review agora carrega `hint`; a UI já renderizava esse campo.
- **S5:** adicionei regressões com o trecho `format!("Bearer {token}")`, `.env.example` com `SMTP_PASSWORD=`, e contra-provas `password=hunter2` + `Authorization: Bearer eyJ...`.

**VERIFICATION**

- `cargo fmt --manifest-path src-tauri/Cargo.toml --check` OK
- `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings` OK
- `cargo test --manifest-path src-tauri/Cargo.toml` OK, `221 passed`
- `corepack pnpm exec tsc --noEmit` OK
- `corepack pnpm test` OK, `226 passed`
- `corepack pnpm exec vite build` OK, com aviso já conhecido de chunks grandes

**Desvios**

- Rodei `corepack pnpm install` porque `node_modules` não existia e o primeiro `tsc` falhou por falta de `vite/client`.
- Fiz um commit local único e atômico para o bug inteiro. Worktree final limpo.

Auto-nota: **9.3/10**.