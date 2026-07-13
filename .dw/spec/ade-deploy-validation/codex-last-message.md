**STATUS**
Implementado em 3 commits locais, sem push. Worktree limpo.

Commits:
- `e0bcce7` `fix: surface ADE plan validation findings`
- `35797cf` `fix: align ADE docker contracts and env templates`
- `2b05957` `chore: apply rustfmt`

**F→Fix→Teste**
- F1.1/F1.2: validador token-aware para `rm -rf /`, `chmod -R 777 /` e placeholders de secret em [deploy_plan.rs](/home/bruno/code/clia-local-ade-fix/src-tauri/src/deploy_plan.rs:964). Testes em [deploy_plan.rs](/home/bruno/code/clia-local-ade-fix/src-tauri/src/deploy_plan.rs:1261).
- F1.3: `validation_findings` tipado, `validation_errors` com path e sem duplicar findings em warnings em [deploy_plan.rs](/home/bruno/code/clia-local-ade-fix/src-tauri/src/deploy_plan.rs:25) e [deploy_plan.rs](/home/bruno/code/clia-local-ade-fix/src-tauri/src/deploy_plan.rs:475). UI renderiza findings e banner inclui motivos em [DeployPackagesPanel.tsx](/home/bruno/code/clia-local-ade-fix/src/DeployPackagesPanel.tsx:657) e [DeployPackagesPanel.tsx](/home/bruno/code/clia-local-ade-fix/src/DeployPackagesPanel.tsx:1032).
- F1.4: detector aceita `Dockerfile.*` e compose ampliado em [deploy_detect.rs](/home/bruno/code/clia-local-ade-fix/src-tauri/src/deploy_detect.rs:271). Packager escolhe Dockerfile com preferência `Dockerfile`, `Dockerfile.prod`, `Dockerfile.dev`, demais em [deploy_package.rs](/home/bruno/code/clia-local-ade-fix/src-tauri/src/deploy_package.rs:927). Evidence files inclui contratos Docker/Compose em [deploy_plan.rs](/home/bruno/code/clia-local-ade-fix/src-tauri/src/deploy_plan.rs:732).
- F1.5: `.env.example` mescla `env.required` e `env.optional` do plano em [deploy_package.rs](/home/bruno/code/clia-local-ade-fix/src-tauri/src/deploy_package.rs:869). `#KEY=` vira optional declarado em [deploy_env.rs](/home/bruno/code/clia-local-ade-fix/src-tauri/src/deploy_env.rs:267).

**VERIFICATION**
- `corepack pnpm install --frozen-lockfile` → exit 0
- `cargo fmt --check` → exit 0
- `cargo clippy --all-targets -- -D warnings` → exit 0
- `cargo test` → exit 0, 211 passed
- `corepack pnpm exec tsc --noEmit` → exit 0
- `corepack pnpm exec vite build` → exit 0
- `corepack pnpm test` → exit 0, 225 passed
- `git diff --check` → exit 0
- `corepack pnpm exec eslint .` → exit 1, 37 issues preexistentes em `src/App.tsx`, fora dos arquivos alterados.

**Evidência**
O teste [real_owner_lettrebox_deploy_plan_fixture_validates_when_available](/home/bruno/code/clia-local-ade-fix/src-tauri/src/deploy_plan.rs:1365) leu o `deploy-plan.json` real de `/home/bruno/wks/letrebox/...` em modo leitura e passou: `validate_plan` retornou `passed`.

**Desvios**
- `cargo fmt` formatou também `docker.rs`, `git.rs`, `lib.rs`, `store.rs`; ficou isolado no commit `2b05957`.
- ESLint não fecha por dívida existente em `src/App.tsx`.

**Auto-nota**
Conformidade 9/10 · Gate 9/10 · Completude 10/10 · Higiene 9/10 · Qualidade 9/10.