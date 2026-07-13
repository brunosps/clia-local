# codex fix — round 3.1 (gate: verify 10, adversarial 5.5 — REPROVADO)

Worktree `/home/bruno/code/clia-local-review-fix` (HEAD `8807f6d`). O gate provou com clone
compilado que o scanner unificado ENFRAQUECEU e endureceu nos lugares errados. Corrija T1–T4;
um commit; sem push. REGRA EXPLÍCITA: é PROIBIDO ajustar/reescrever fixtures de teste para
contornar um caso — no round anterior o fixture `PASSWORD={password}` foi trocado por `PASSWORD=`
para o teste passar; restaure e trate o caso de verdade.

## T1 — HIGH: só a PRIMEIRA ocorrência de cada marcador é analisada (`deploy_scan.rs:12-15`)

`next_secret_marker(offset)` é vestigial — chamado só com 0. Placeholder no topo esconde segredo
real depois: `SMTP_PASSWORD=\nADMIN_PASSWORD=hunter2` PASSA (os scanners antigos, plano E pacote,
bloqueavam com while-loop de offset). **Fix**: iterar TODAS as ocorrências por marcador (offset
avançando até esgotar) — bloqueia se QUALQUER uma for valor real. Vale pro `bearer ` também
(`format!("Bearer {token}")` no topo + `Bearer eyJhbGciOi...` embaixo tem que bloquear). Testes
multi-ocorrência nos DOIS scanners, incluindo o próprio fixture `.env.example` do round + valor
real no final (o contrato S1 dizia exatamente isso).

## T2 — MEDIUM: restaurar rebaixamento de paths não-runtime (`deploy_package.rs:672`)

O scan antigo rebaixava `test/tests/__tests__/e2e/fixtures/docs` para WARNING não-bloqueante
(`is_non_runtime_path`); o novo ignora o path e BLOQUEIA — teste com `password=hunter2` (padrão
comum, inclusive neste próprio repo) trava o pacote. Restaure o rebaixamento no scanner de PACOTE
(no de PLANO não se aplica — artefatos de plano são sempre runtime). Teste dos dois lados.

## T3 — MEDIUM: `password={var}` é template de código, não segredo (`deploy_scan.rs:63`)

`bearer_value_is_blocking` já trata `{` como construção de string; `secret_assignment_is_placeholder`
não — `format!("password={password}")` bloqueia (o scanner de pacote antigo tratava). Unifique:
token começando com `{` = placeholder nos assignments também. RESTAURE o fixture original
`PASSWORD={password}` no teste que foi reescrito.

## T4 — LOW: `bearer` no fim de linha lê a linha seguinte (`deploy_scan.rs:95`)

`tail.trim_start()` engole o `\n` — `Bearer ` no fim da linha + linha seguinte começando com
palavra ≥8 chars bloqueia indevidamente. Corte o tail na primeira quebra de linha ANTES do trim
(como o caminho de assignment já faz). Teste.

## Gate

`cargo fmt --check` · `clippy -D warnings` · `cargo test` · `tsc --noEmit` · `pnpm test` ·
`vite build`. Auto-nota; repita enquanto <9. Relatório: STATUS · T→fix→teste · VERIFICATION ·
confirmação explícita de que NENHUM fixture foi afrouxado · auto-nota.
