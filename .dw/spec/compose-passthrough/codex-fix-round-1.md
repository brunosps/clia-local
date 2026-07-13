# codex fix — passthrough/prefill round 1 (gate: verify 10, adversarial REPROVOU com 1 CRITICAL)

Worktree `/home/bruno/code/clia-local-passthrough` (HEAD `4926ada`). O passthrough e o grid estão
bons; o PREFILL criou um tiro no pé. Corrija Y1–Y5; um a dois commits; sem push.

## Y1 — CRITICAL: o prefill escreve defaults no `.env.example` do pacote e o PRÓPRIO scanner bloqueia o pacote

`write_env_example` (deploy_package.rs:1131) grava `POSTGRES_PASSWORD=rwfw`,
`BULWARK_ADMIN_PASSWORD=...`, `BULWARK_SESSION_SECRET=change-me-...`; `scan_package_review_files`
(:2063) escaneia esse arquivo depois e `rwfw` não é placeholder → finding BLOQUEANTE → deploy
recusado (reproduzido empiricamente com o compose.deploy.yaml REAL do lettrebox). O cenário de
aceite ("zero pendências, zero digitação") produz pacote UNDEPLOYABLE.
**Fix**: o `.env.example` GERADO PELO ADE é artefato próprio derivado de fontes já públicas do
pacote — **não deve ser escaneado** (exclua-o de `scan_package_review_files`; continue escaneando o
`.env.example` DO PROJETO copiado em `projects/<slug>/source/`, esse é conteúdo do repo do dono).
Justifique no código com comentário curto. **Teste obrigatório de fluxo real**: `create_package`
com o compose.deploy.yaml REAL do lettrebox (copie o arquivo do repo como fixture) → pacote SEM
findings bloqueantes E `.env.example` com os defaults → `approve_version` passa.

## Y2 — MEDIUM: pacotes legados — placeholder antigo vira "default do projeto"

`build_environment`/`read_env_template_variables` tratam QUALQUER valor não-vazio do template como
default → nos pacotes antigos (deploy-001/003 do dono) `DATABASE_URL=postgres://user:password@...`
(placeholder gerado pelo ADE velho) vira "default do projeto", fica ready sem save e VAI PRO RUNTIME.
**Fix**: distinguir a PROVENIÊNCIA — só valores marcados como default do projeto (nova geração,
com `default_source`) contam; template legado sem marcação = placeholder (campo vazio, pendente).
Marque no `.env.example` gerado (ex.: comentário-cabeçalho de schema/versão do template, ou linha
`#dw:default KEY`) — escolha o mecanismo mais simples e à prova de leitura antiga. Teste com o
`.env.example` REAL do deploy-003 (`/home/bruno/wks/letrebox/.dw/deploy-packages/4/lettrebox-deploy/deploy-003/.env.example`)
→ nenhuma chave vira default.

## Y3 — MEDIUM: stack multi-projeto no passthrough

`package_compose_mode` (:851) usa só o compose do PRIMEIRO projeto — os demais somem sem aviso.
**Fix**: se >1 projeto tem compose próprio, NÃO faça passthrough silencioso: gere warning
bloqueante-informativo no plano/manifest e caia no compose gerado (ou peça artefato do agente).
Teste dos dois casos (1 projeto → passthrough; 2 → fallback + warning).

## Y4 — LOW: regex de defaults do compose

Ignorar linhas comentadas (`#`), ignorar `$${...}` (escape do compose), suportar `${VAR-default}`
(sem `:`), e default contendo `}` aninhado não deve truncar. Testes dos 4 casos.

## Y5 — LOW: defaults OPCIONAIS no runtime + `#KEY=valor` do projeto

`write_runtime_env` injeta toda var com valor — inclusive opcional com default — e
`read_project_env_example_defaults` trata `#KEY=value` comentado no `.env.example` DO PROJETO como
declaração ativa. **Fix**: opcional com default entra no runtime só se o compose realmente a
interpola (ou simplesmente NÃO injetar opcionais não-salvos — o compose já tem o default embutido);
`#KEY=value` do projeto = comentário (não é default). Testes.

## Fence e gate

Worktree only; sem push; sem binaries/. Gate: `cargo fmt --check` · `clippy -D warnings` ·
`cargo test` (com os testes de fluxo real do Y1/Y2 usando os ARQUIVOS REAIS citados) · `tsc
--noEmit` · `pnpm test` · `vite build`. Auto-nota; <9 repete. Relatório: STATUS · Y→fix→teste ·
VERIFICATION · auto-nota.
