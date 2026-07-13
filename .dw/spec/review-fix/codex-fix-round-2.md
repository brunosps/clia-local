# codex fix — round 3.2 PONTUAL (re-gate: 7.0; T1/T3/T4 fixed, T2 abriu HIGH novo)

Worktree `/home/bruno/code/clia-local-review-fix` (HEAD `cf7ad3a`). O núcleo do scanner está
sólido (matriz de 77 casos passou). Falta UM fix cirúrgico + higiene de teste. Um commit, sem push.

## U1 — HIGH: `is_non_runtime_path` sobre path ABSOLUTO = bypass integral do gate

`deploy_package.rs:682` avalia `source_path`/`copied_path` absolutos; o `copied_path` SEMPRE contém
o slug da STACK (controlado pelo usuário) e do projeto: stack chamada "Test" → componente `test` no
path → TODOS os findings do pacote (incluindo `scan_package_review_files:1772` — .env.example e
compose) viram warning não-bloqueante. Idem projeto "Docs", checkout sob `~/test/…`.
**Fix**: casar componentes RELATIVOS — `strip_prefix` da raiz de origem no `source_path` e de
`projects/<slug>/source` no `copied_path` — antes do match por componente. Nunca o absoluto.
Testes: (a) FLUXO REAL (não a função isolada): pacote com stack slug `test` e segredo real em
`src/` → BLOQUEIA; (b) arquivo sob `Tests/` (mixed-case, dentro do projeto) → warning; (c) segredo
em `src/` de stack normal → bloqueia (regressão).

## U2 — LOW (higiene): teste do rebaixamento deve exercitar o fluxo, não a função

O teste atual chama `scan_secret_content` direto com path fabricado (confiança falsa). Ajuste para
passar pelo caminho real (ou adicione teste de integração do create_package/scan de review com a
árvore montada). Nota: para dirs de nome exato o `should_exclude_name` já exclui antes — o
rebaixamento só vale para variantes (mixed-case etc.); deixe isso registrado em comentário curto no
código para o próximo leitor.

## Gate

`cargo fmt --check` · `clippy -D warnings` · `cargo test` · `tsc --noEmit` · `pnpm test` ·
`vite build`. Auto-nota; <9 repete. Relatório: STATUS · U→fix→teste · VERIFICATION · auto-nota.
