**STATUS**  
Fechado em um commit local, sem push: `412d72c fix: harden deploy review dismissal inheritance`. Worktree limpo.

**X → fix → teste**  
- X1: herança agora decide pela timeline de audit da stack inteira; restore usa o mesmo guard de status do dismiss. Teste cobre restore em v1 bloqueando v3 e novo dismiss em v3 herdando em v4.
- X2: duplicatas idênticas recebem `occurrence_index/count`; seleção não falha se a identidade é a mesma e dismiss/restore atuam em lote. UI mostra “2 ocorrências”. Teste cobre dismiss e restore de ambas.
- X3: fallback `path+reason` só vale para findings sem conteúdo; secret-content sem `marker+sha` não casa. Backend e parser TS alinhados. Testes Rust/TS adicionados.
- X4: `unsupported deploy strategy` usa path relativo do pacote (`projects/<slug>/source`). Teste adicionado.
- X5: `update_deploy_version_review_json` usa `TransactionBehavior::Immediate` e `busy_timeout`; teste de pragma adicionado.

**VERIFICATION**  
- `cargo fmt --manifest-path src-tauri/Cargo.toml --check` passed  
- `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings` passed  
- `cargo test --manifest-path src-tauri/Cargo.toml` passed: 238 tests  
- `corepack pnpm exec tsc --noEmit` passed  
- `corepack pnpm test` passed: 26 files, 230 tests  
- `corepack pnpm exec vite build` passed, only existing large-chunk warning

Auto-nota: 9.3/10.