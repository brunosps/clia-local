# codex — clia-local: passthrough determinístico do compose do projeto (custom_compose)

Worktree `/home/bruno/code/clia-local-passthrough` (branch `fix/compose-passthrough`, base 0.2.3
main). BUG REAL (deploy-003 do dono): estratégia `custom_compose`, o agente declarou "reuses the
project's compose verbatim" mas emitiu `artifacts.compose: null` → o packager caiu no compose
GERADO (1 serviço, sem postgres/mailhog/stalwart) → deploy quebraria o app. Confiar no LLM pra
copiar o compose do repo é loteria (no deploy-001 ele copiou; no deploy-003 não). Commits atômicos,
sem push.

## P1 — Detector registra QUAL compose casou

`deploy_detect.rs`: além de `has_compose`, gravar `compose_path` (relativo à raiz) no
`DeployProjectDetection` com prioridade: `compose.deploy.*` > `docker-compose.deploy.*` >
`docker-compose.yml|yaml`/`compose.yml|yaml` > `*.prod.*` > variantes dev por último (dev compose
costuma não ter o app). Propagar pro project-context (o agente planejador passa a VER qual arquivo é
o contrato).

## P2 — Packager: passthrough quando o agente não emite compose

Em `create_package`, estratégia `custom_compose` + `compose_path` detectado + plano SEM
`artifacts.compose`: NÃO escrever o compose gerado. Em vez disso, os scripts do runbook
(deploy/stop/logs/healthcheck em `write_scripts`) invocam o compose DO SOURCE copiado:
`docker compose --env-file ./.env -f projects/<slug>/source/<compose_path> -p <project> ...`
(rodando do package root). Racional a VALIDAR com teste real (`docker compose config`):
- paths RELATIVOS no compose resolvem relativos ao DIRETÓRIO DO ARQUIVO compose (spec v2) → build
  context `${VAR:-.}` e bind mounts do repo funcionam sem env extra;
- `--env-file ./.env` (package root) alimenta a INTERPOLAÇÃO `${...}` sem precisar conhecer nomes
  de variável do projeto (se ./.env não existir no momento do script, omitir a flag — guard no sh).
Se a validação empírica contradisser o racional (versão do compose etc.), ajuste a mecânica
(ex.: copiar o compose pro package root reescrevendo contexts) e documente com evidência.
Teste de integração com fixture ESTILO LETTREBOX REAL: compose na raiz com
`build: {context: ${SRC:-.}, dockerfile: Dockerfile.prod}` + bind mount `${SRC:-.}/config/x.json` +
multi-serviço → `docker compose config` do package resolve contexts/mounts para dentro de
`projects/<slug>/source/` e lista TODOS os serviços.

## P3 — Coerência de env e validação

- S3 (chaves do detector opcionais) passa a valer TAMBÉM no passthrough (contrato próprio presente,
  mesmo sem artifacts.compose do agente).
- `validate_plan`: `custom_compose` + repo com compose + `artifacts.compose: null` = OK (o
  passthrough cobre) — mas adicionar um warning INFORMATIVO no plano ("package will passthrough
  <compose_path>") pra ficar auditável; e o prompt do planejador ganha a instrução explícita: "se o
  projeto fornece compose (campo compose_path do contexto), você PODE omitir artifacts.compose — o
  ADE fará passthrough — ou emitir uma versão adaptada; NUNCA descreva reuso sem fazer uma das duas".
- UI: o card "Plano" mostra qual compose será usado (passthrough vs artefato do agente vs gerado).

## P4 — UX menor (lição do dono hoje): placeholder ≠ valor

No painel de ambiente, os inputs obrigatórios usam placeholder com cara de valor válido
(`postgres://user:password@...`) — o dono clicou salvar sem digitar e salvou vazio. Adicionar botão
"Usar exemplo" por campo obrigatório (preenche o input com o placeholder de verdade) e/ou helper
curto "o texto cinza é exemplo — digite ou clique em usar". Salvar com obrigatória vazia continua
permitido (save parcial), mas o toast/resultado deve dizer "salvo; N obrigatórias continuam vazias".

## Fence e gate

Worktree only; sem push; sem binaries/. Gate: `cargo fmt --check` · `clippy -D warnings` ·
`cargo test` (incluindo o teste de integração do P2 com `docker compose config` REAL — docker está
disponível) · `tsc --noEmit` · `pnpm test` · `vite build`. Auto-nota; <9 repete. Relatório:
STATUS · P→fix→teste · VERIFICATION (com o output do compose config da fixture) · auto-nota.
