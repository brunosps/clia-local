# codex — clia-local round 2: healthcheck objeto + HINTS de correção nos findings

Worktree `/home/bruno/code/clia-local-ade-fix` (branch `fix/ade-healthcheck-hints`, base `c956dbc`
v0.2.1). Bug REAL do dono (2º deploy bloqueado) + feature pedida por ele. Commits atômicos, sem push.

## Contexto (run real)

O plano do agente para o lettrebox foi bloqueado com "web or compose deploy plans must include at
least one healthcheck" — MAS o plano TEM healthcheck, como OBJETO:
`projects[0].healthcheck = {"type":"http","url":"http://127.0.0.1:8080/health",...}`.
Causa: `plan_healthchecks` (`src-tauri/src/deploy_plan.rs:855-871`) só extrai `Value::as_str`.
O plano real está em `.dw/spec/ade-healthcheck-hints/blocked-plan-fixture.json` (nesta pasta) —
use como fixture de regressão.

## R1 — Aceitar healthcheck string OU objeto

`plan_healthchecks`: além de string não-vazia, aceitar objeto com `url` ou `command`/`test`
não-vazios (extraia a URL/comando como representação string; objeto vazio/sem esses campos NÃO
conta). Vale no top-level e por projeto. Atualize também o PROMPT do planejador
(`deploy_plan_prompt`, ~:340-400): documentar no shape que `healthcheck` aceita
`"curl ..."/"http://..."` (string) OU objeto `{"type":"http","url":...,"interval_seconds":...}` —
o agente não pode ter que adivinhar. Testes: string passa (regressão), objeto do fixture passa,
objeto vazio/null não conta, `{}` não conta.

## R2 — HINTS de correção em TODOS os findings bloqueantes (pedido explícito do dono: "dar direcionamento no erro para que possamos planejar as correções")

- Rust: campo `hint: String` no finding (`finding()` helper e struct tipada `validation_findings`
  do DeployPlanReport). Para CADA check de `validate_plan`, um hint ACIONÁVEL específico. Exemplos
  do tom (escreva todos, PT-BR, uma frase objetiva):
  - healthcheck ausente → "Inclua `projects[].healthcheck` (string de comando/URL ou objeto
    {url, interval_seconds, ...}) ou um healthcheck top-level no plano."
  - strategy mismatch → "A estratégia detectada para esta seleção é `<X>`; regenere o plano ou
    ajuste a seleção de projetos."
  - dangerous command → "Remova ou ajuste o comando apontado no artefato; alvos na raiz (`rm -rf /`)
    são bloqueados — use paths específicos."
  - secret marker → "Troque o valor literal por placeholder `${VAR}` e declare a chave em
    `env.required` para preenchê-la na UI de deploy."
  - script path fora de scripts/ → "Mova o script para `scripts/<nome>.sh` — só esse diretório é
    permitido."
  - confidence low → "O agente declarou confiança baixa; revise o contexto do projeto (evidence
    files) e replaneje."
  - schema_version / project_ids / desktop_dev target / script vazio → hints equivalentes.
- `validation_errors()`: formato `"<path>: <reason> — <hint>"` (o hint viaja também no texto).
- Front (`DeployPackagesPanel.tsx` card de findings + banner; `types.ts`): renderizar o hint sob o
  reason (estilo secundário/menor, classe própria coerente com o painel). Banner mantém truncagem.
- Testes Rust dos hints (cada check emite hint não-vazio) + teste TS se houver padrão de teste do
  painel (senão só typecheck).

## R3 — Regressão com o plano real

Teste incondicional: `blocked-plan-fixture.json` (copie para `src-tauri/tests/fixtures/` com nome
descritivo, ex. `lettrebox-deploy-plan-object-healthcheck.json`) + a detecção real do lettrebox
(custom_compose) → `validate_plan` retorna `passed`, zero findings.

## Fence e gate

Iguais aos rounds anteriores (worktree only; sem push; sem tocar `src-tauri/binaries/`; não rodar o
app). Gate: `cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo test` ·
`corepack pnpm exec tsc --noEmit` · `corepack pnpm test` · `corepack pnpm exec vite build`.
Auto-nota 0–10; repita enquanto <9. Relatório: STATUS · R→fix→teste · VERIFICATION · desvios ·
auto-nota por critério.
