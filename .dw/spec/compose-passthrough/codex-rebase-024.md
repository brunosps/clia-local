# codex — rebase do passthrough/prefill em cima do 0.2.4 (override de findings) + reconciliação

Worktree `/home/bruno/code/clia-local-passthrough` (branch `fix/compose-passthrough`, HEAD
`648945c`, baseada no 0.2.3). A main do clia-local avançou para **0.2.4** (feature de override:
"Aceitar risco" por finding, identidade de finding = path relativo + marker + sha256 da linha,
tombstone de stack, auditoria append-only). Os dois trabalhos tocam `deploy_package.rs`,
`deploy_env.rs`, `deploy_scan.rs`, `DeployPackagesPanel.tsx`, `deploy.ts`, `styles.css` → o rebase
conflita. Faça o rebase e reconcilie. Um commit final coeso é aceitável (ou a cadeia rebasada), sem
push.

## Tarefa

1. `git rebase main` (ou rebase interativo squashando os commits do branch) resolvendo TODOS os
   conflitos com JUÍZO — não escolha "ours/theirs" cego:
   - a semântica do 0.2.4 (identidade de finding com marker+sha, dismiss/restore/audit, path
     relativo) tem que ficar INTACTA;
   - a semântica do passthrough/prefill (compose_path, SourcePassthrough, defaults `#dw:default`,
     `.env.example` gerado fora do scan, grid empilhado) tem que ficar INTACTA;
   - onde as mudanças competem no MESMO trecho (ex.: `scan_package_review_files`, `write_env_example`,
     render dos findings/env no painel), componha as duas — o resultado tem que satisfazer os testes
     dos DOIS lados.
2. **Reconciliação de produto** (decisão já tomada, implemente): o `.env.example` GERADO pelo ADE
   continua FORA do scan (Y1) — a razão é que é artefato derivado, não conteúdo do dono; o override
   0.2.4 existe para os findings que restam (conteúdo do repo). Garanta que os dois convivem: um
   finding de secret-content num arquivo do REPO continua bloqueando e é dismissível.
3. Suíte completa dos dois lados verde (o rebase não pode "resolver" conflito deletando teste).

## Gate

`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo test` (TODOS os testes
das duas frentes: passthrough com docker compose config real, prefill com fixtures reais do
lettrebox, dismiss/restore/herança/auditoria do 0.2.4) · `tsc --noEmit` · `pnpm test` ·
`vite build`. Bump de versão para **0.2.5** nos três arquivos (package.json, tauri.conf.json,
src-tauri/Cargo.toml — o guard de sync exige). Auto-nota; <9 repete.
Relatório: STATUS · conflitos resolvidos (arquivo → como) · VERIFICATION · auto-nota.
