# codex — clia-local round 3: scanner de review do pacote placeholder-aware + env do detector opcional sob compose próprio

Worktree `/home/bruno/code/clia-local-review-fix` (branch `fix/review-scanner-placeholders`).
Bug REAL (3º deploy do dono, pacote deploy-001 criado e travado no review). Commits atômicos, sem push.

## Contexto (findings reais do run — use como fixtures)

`deploy_versions.blocking_findings_json` do pacote real:
1. [error] `projects/lettrebox/source/crates/mail-driver/src/stalwart/client.rs` — "secret-like
   content marker `bearer `" → FALSO-POSITIVO: código-fonte montando header (`format!("Bearer {…}")`).
2. [error] `projects/lettrebox/source/.env.example` — "secret-like content marker `password=`" →
   FALSO-POSITIVO: linhas placeholder VAZIAS (`SMTP_PASSWORD=`).
O validador de PLANOS já ganhou análise por-ocorrência placeholder-aware (F1.2,
`secret_assignment_is_placeholder` em deploy_plan.rs) — o scanner de review do PACOTE
(deploy_package.rs, scan do copy_source_snapshot/review findings) ainda usa substring cru. Mesma
classe, caminho de código diferente.

## S1 — Scanner de review por-ocorrência, placeholder-aware

Reuse/extraia os helpers do F1.2 (não duplique lógica — módulo compartilhado): `password=`/`secret=`
/`api_key=`/`apikey=` só bloqueiam com VALOR literal real; vazio/`${...}`/`$VAR`/`<placeholder>`/
`xxx`/`changeme` passam. Arquivos de template (`.env.example`, `.env.template`, `.env.sample`):
mesmo tratamento (placeholder passa; um valor real neles CONTINUA bloqueando).

## S2 — `bearer ` occurrence-aware

`bearer ` seguido de construção de string/var (`{`, `"`, `'`, `$`, fim de linha, `%s`) = código
montando header → passa. Seguido de token literal (≥8 chars alfanumérico/base64-like) = segredo →
bloqueia. Aplique nos DOIS scanners (plano e pacote) se o do plano tiver o mesmo defeito — confira.

## S3 — Chaves do detector viram OPCIONAIS quando o plano traz compose próprio

Quando o plano do agente fornece `artifacts.compose` (contrato próprio — caso custom_compose do
lettrebox), as chaves semeadas pelo detector (`DATABASE_URL`/`SMTP_URL`/`REDIS_URL`) entram no
`.env.example` como OPCIONAIS (`#KEY=`), não obrigatórias — o compose próprio não as consome e a UI
hoje exige preencher valores que nada lê. Sem compose do agente (ADE gera o compose), continuam
required como hoje. Teste dos dois modos.

## S4 — Hints também nos findings de review

Os findings de review (blocking_findings_json / UI "Bloqueios do review") ganham `hint` acionável
como os do validador de plano (mesmo padrão do R2 anterior): ex. secret marker → "Se for
placeholder/código, ajuste o padrão; se for segredo real, remova do source e use env"; env file
excluído → já é warning, hint "esperado — preencha na UI de ambiente". UI renderiza (mesmo estilo).

## S5 — Regressão com os casos reais

Fixtures: (a) trecho real do client.rs (linha do Bearer) e (b) o .env.example gerado do pacote →
review SEM findings bloqueantes; (c) contra-prova: arquivo com `password=hunter2` e `Authorization:
Bearer eyJhbGciOi...` → AMBOS bloqueiam. Se possível, teste de integração do create_package com uma
árvore mínima contendo os dois falsos-positivos → review passa.

## Fence e gate

Iguais aos rounds anteriores (worktree only; sem push; sem binaries/; não rodar o app). Gate:
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo test` · `corepack pnpm
exec tsc --noEmit` · `corepack pnpm test` · `corepack pnpm exec vite build`. Auto-nota; repita
enquanto <9. Relatório: STATUS · S→fix→teste · VERIFICATION · desvios · auto-nota.
