# codex — clia-local: fixes do pipeline ADE (validador, UX de erro, detector, env)

Worktree: `/home/bruno/code/clia-local-ade-fix` (branch `fix/ade-deploy-validation`, base main
`10666b8`). App Tauri: front Vite/TS em `src/`, backend Rust em `src-tauri/src/`. Implemente
F1.1–F1.5 abaixo com testes; commits atômicos por grupo coerente, sem push.

## Contexto (bug real do dono)

O deploy do projeto lettrebox parou com `status: blocked` sem motivo visível: o Dockerfile gerado
pelo agente planejador tinha `rm -rf /var/lib/apt/lists/*` e `contains_dangerous_command`
(`src-tauri/src/deploy_plan.rs:903-914`) casa `rm -rf /` por SUBSTRING. Artefatos reais do caso:
`/home/bruno/wks/letrebox/.dw/deploy-plans/plan-1783900982742714445/analysis/` (leia o
`deploy-plan.json` e o `validation-report.json` — use o Dockerfile de lá como fixture de regressão).

## F1.1 — Validador: dangerous command token-aware (`deploy_plan.rs:903-914`)

`rm -rf /` só é perigoso quando o alvo é a RAIZ: seguido de fim-de-texto, whitespace, `"`, `'`,
`;`, `&`, `|` — ou `rm -rf /*`. Paths reais (`rm -rf /var/lib/apt/lists/*`, `rm -rf /tmp/build`)
NÃO bloqueiam. Mesma revisão para `chmod -r 777 /` (o check é lowercase — cobre `-R`). Os demais
marcadores (`mkfs.`, `dd if=`, `/etc/shadow`) ficam como estão. Unit tests: apt cleanup passa;
`rm -rf /`, `rm -rf /*`, `rm -rf / ` bloqueiam; caso de regressão com o Dockerfile REAL do plano
bloqueado do dono (fixture) → `validate_plan` retorna `passed`.

## F1.2 — Validador: secret marker tolerante a placeholder (`deploy_plan.rs:896-901`)

`password=`/`secret=`/`api_key=`/`apikey=` hoje bloqueiam qualquer ocorrência, inclusive
placeholders legítimos. Tolerar quando a atribuição é claramente placeholder: valor vazio
(`KEY=` no fim de linha), `${...}`, `$VAR`, `<placeholder>`, `xxx`/`changeme`
(case-insensitive). Continuar bloqueando valor literal real (`password=hunter2`). Analise por
OCORRÊNCIA (cada match decide pelo que vem depois), não pelo texto inteiro. `bearer ` segue igual.
Unit tests dos dois lados.

## F1.3 — UX: motivo detalhado do bloqueio (reclamação explícita do dono: "vem só um block")

Rust (`src-tauri/src/deploy_plan.rs`):
- Novo campo tipado em `DeployPlanReport` (struct ~:24-46): `validation_findings:
  Vec<DeployPlanFinding>` com `{path, reason, severity, blocking}` (serialize camelCase igual ao
  resto do payload — confira a convenção do struct).
- `validation_errors()` (:471-489): incluir o path — formato `"<path>: <reason>"`.
- `report_from_bundle` (:185-192): REMOVER a duplicação dos reasons em `warnings` (hoje a UI
  mostra cada motivo 2x).
Front:
- `src/types.ts` (~:160-181): tipar `validation_findings` no `DeployPlanReport`.
- `src/DeployPackagesPanel.tsx` (~:1014-1038): renderizar a lista de findings no card do plano —
  path + reason + badge de severity (reuse as classes/padrões visuais existentes do painel;
  `deploy-warning` é a classe atual). Sem findings → comportamento atual.
- `:657-659` + `src/deploy.ts` (~:521): quando `status !== 'passed'`, o banner de erro inclui os
  motivos (primeiros N com path), não só a mensagem genérica "O plano do agente não passou na
  validação". Mantenha PT-BR consistente com as outras mensagens do `deployErrorMessage`.

## F1.4 — Detector: contrato Docker real (`src-tauri/src/deploy_detect.rs:121-125`)

- `has_dockerfile`: além de `Dockerfile`/`Dockerfile.dev`, aceitar `Dockerfile.prod` e qualquer
  `Dockerfile.*` na raiz (scan de dir simples, case-sensitive).
- `has_compose`: além dos 4 nomes atuais, as variantes `.yaml` de todos, mais
  `compose.dev.yaml|yml`, `compose.prod.yaml|yml`, `compose.deploy.yaml|yml`,
  `docker-compose.deploy.yaml|yml`.
- **Knock-on no packager** (`src-tauri/src/deploy_package.rs:843-856`): a seleção de Dockerfile do
  repo em `custom_compose` considera só `Dockerfile`/`Dockerfile.dev` — alinhar com o detector
  (ordem de preferência: `Dockerfile`, `Dockerfile.prod`, `Dockerfile.dev`, demais `Dockerfile.*`).
- **Evidence files** (`deploy_plan.rs`, `build_project_context` ~:507+ e a lista de key files
  ~:703-777): garantir que `Dockerfile.*` e `compose*.y(a)ml`/`docker-compose*.y(a)ml` presentes no
  repo entram nos `evidence_files` (respeitando os limites de 40 arquivos/48KB existentes).
- Tests: fixture de projeto com `Dockerfile.prod` + `compose.yaml` → `has_dockerfile=true`,
  `has_compose=true`, estratégia `custom_compose`.

## F1.5 — Env do plano entra no `.env.example` do package

- `write_env_example` (`deploy_package.rs:872-890`): receber o plan bundle e mesclar — chaves de
  `env.required` do plano como `KEY=` (required), `env.optional` como `#KEY=` (comentada);
  preservar as chaves por serviço detectado (DATABASE_URL/REDIS_URL/SMTP_URL) sem duplicar;
  ordenação estável.
- `read_template_variables` (`src-tauri/src/deploy_env.rs:180-194`): tratar linha `#KEY=` como
  chave DECLARADA-OPCIONAL — `save_environment` aceita, `require_environment_ready` não exige.
- Unit tests: plano com required+optional → example correto; save de chave optional passa; ready
  não bloqueia por optional ausente; chave não-declarada continua rejeitada.

## Fence

Só esta worktree. NÃO toque em `src-tauri/binaries/` (winbox/rtk), `scripts/prepare-*.mjs`,
`CLAUDE.md`/`AGENTS.md`; NÃO push; NÃO rode o app Tauri (sem sessão gráfica do dono); NÃO mexa em
`/home/bruno/wks/` (só LEITURA dos artefatos do plano bloqueado). Rede liberada.

## Gate interno (reporte exits)

No `src-tauri/`: `cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo test`.
Na raiz: `corepack pnpm install --frozen-lockfile` (se preciso) · typecheck do front
(`corepack pnpm exec tsc --noEmit` ou o script do package.json) · `corepack pnpm exec eslint .`
se o repo tiver lint configurado · `corepack pnpm exec vite build` (prova que o front compila).
Auto-nota 0–10 (conformidade/gate/completude/higiene/qualidade); corrija e repita enquanto <9.

## Relatório final

STATUS · F→fix→teste (com arquivo:linha) · VERIFICATION (comando→exit) · evidência: fixture real
do dono passa na validação; findings aparecem tipados no payload · desvios · auto-nota por critério.
