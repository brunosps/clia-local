# CLAUDE.md — clia.local

Contexto para um agente de IA continuar este projeto. Leia isto antes de mexer no código.

## O que é

`clia.local` é a versão **100% local** do app desktop CLIA. **Sem nuvem, sem portal, sem
pareamento de device** — tudo roda na máquina (SQLite + git CLI + agentes locais).

Foi criado como **fork enxuto** de `../clia-wks/app/` (o app desktop do monorepo
`clia-wks`, que tinha portal na nuvem). A camada de nuvem foi **arrancada por completo** e
as features que dependiam do portal foram re-localizadas. **Não há código compartilhado
vivo com o monorepo** — é uma cópia independente.

> Regra de ouro: **não reintroduza nuvem.** Nada de HTTP para portal, sync, device login,
> websockets, analyze/opportunities. Se uma feature precisar de dados, ela vem do SQLite
> local, do git local ou de um processo de agente local.

## Stack & layout

Tauri 2 · React 19 · TypeScript · Rust. Frontend em `src/`, backend Rust em `src-tauri/`.

| Caminho | Papel |
|---------|-------|
| `src/App.tsx` | **Monolito de ~11k linhas** — quase toda a UI e o estado vivem aqui. |
| `src/tauri.ts` | Camada `api.*`: wrappers de `invoke` para cada comando Tauri. Todos passam por `invokeSafe` → retorna `{ ok: true, value } \| { ok: false, error }` (nunca lança). |
| `src/types.ts` | Tipos TS compartilhados (espelham os structs Rust). |
| `src/queue.ts` | Fila de tarefas (kanban): `buildQueue()` sobre `RequirementCard[]`, `QUEUE_BUCKETS`, `statusBucket`, `parseChecklist`/`serializeChecklist`, `bucketCanonicalStatus` + tipos `QueueCard`/`QueueBucket`. |
| `src/source/`, `src/monaco/`, `src/lsp/` | Editor de código (Monaco), LSP. |
| `src/*.test.ts(x)` | Testes Vitest (lógica pura: queue, diff, git graph, etc.). |
| `src-tauri/src/lib.rs` | ~5k linhas: registro de **todos** os `#[tauri::command]` no `generate_handler!` + `app_data_dir()`. |
| `src-tauri/src/store.rs` | ~6.5k linhas: SQLite (rusqlite, bundled). Schema + migrations + queries. |
| `src-tauri/src/git.rs` | Git via **git CLI nativo** (`Command::new("git")`), não libgit2. |
| `src-tauri/src/agent.rs` | Spawn de Codex/Claude/Copilot como processos; streaming via eventos Tauri (`agent://event` etc.). |
| `src-tauri/src/deploy*.rs`, `machine.rs`, `winbox_provider.rs` | Deploy local em VMs (Winbox). Pesado (~10k linhas). |
| `src-tauri/src/terminal.rs`, `lsp.rs`, `rtk.rs`, `solution.rs` | PTY, language server, runtime toolkit, import/export de solução. |

**Persistência:** SQLite em `app_data_dir()` =
`$DW_GUI_HOME` → `app.path().app_data_dir()` (`~/.local/share/dev.clia.local/`) →
`~/.local/share/clia-local/` → `./.clia-local`. Arquivo: `clia-local.sqlite3`
(`store.rs::Database::open`). Preferências de UI ficam na tabela `app_state`
(`get_app_state`/`set_app_state`).

## Sidebar (6 tabs)

`navItems` em `App.tsx` (perto da linha ~470). Render: `activeTab === "..."` no JSX principal.

`queue` · `code` · `git` · `deploy` · `agents` · `settings`.

Removidos do fork: **knowledge** e **skills** — fora do `navItems` **e** do union `Tab`
(em `App.tsx`) e do `TabPreference`/`VALID_TABS` (em `uiPreferences.ts`; preferências
antigas `knowledge`/`skills` migram para `queue`).

## A Fila de tarefas (kanban local)

A aba **Queue** é um quadro **kanban de tarefas locais**, escopado ao **workspace ativo**.
Não há nuvem em nenhuma parte do fluxo.

- **Carregamento:** `loadWorkspaceTasks()` (em `App.tsx`) lê
  `api.listRequirementCards(activeWorkspace.id)`; o board mostra só o workspace ativo. A
  lista de projetos vem do estado `projects`. (Substituiu o antigo `WiredCloudBootstrap`
  sintético, que foi removido.)
- **Board:** `QueuePanel` → 4 colunas fixas **A fazer / Fazendo / Validando / Feito**
  (`QUEUE_BUCKETS`) com **drag-and-drop** (HTML5) entre colunas. Mover status é **manual
  (humano)** — o agente nunca avança o card sozinho. **Arquivar** via
  `api.archiveRequirementCard`.
- **`buildQueue` (`queue.ts`):** mapeia `status → bucket` (tolerante a status legados via
  `statusBucket`), exclui arquivados, filtra por projeto e ordena por prioridade. Os status
  canônicos escritos no drag são `todo`/`doing`/`validating`/`done` (`bucketCanonicalStatus`).
- **Modelo de dados:** reaproveita a tabela `requirement_cards` + colunas novas
  `priority`, `checklist_json`, `agent_prompt` (migração additiva via `ensure_column`).
  Anexos ficam em `requirement_attachments` (já existia), copiados para
  `<workspace>/.dw/gui/attachments/<card_id>/` — **fora dos repositórios de projeto**, então
  não sujam o git de nenhum projeto.
- **TaskModal (`App.tsx`):** clicar num card abre um modal com título, descrição (`body`),
  **checklist** de subtarefas, prioridade (Alta/Média/Baixa), **projetos** (≥1), status,
  **prompt do agente** e **anexos**. Botão **"Executar com agente"** com seletor de agente
  (quando há >1 profile) e **streaming inline** (filtra `agent://event` pela sessão lançada
  via `sendAgentPrompt`, que roda no projeto ativo). Salvar persiste por
  `api.updateRequirementCard` + `api.setRequirementCardProjects` + (se o status mudou)
  `api.updateRequirementCardStatus`.
- **Criar tarefa:** `createQueueCard()` → `api.createRequirementCard` (ID público reservado
  **localmente** por `store.rs::reserve_next_public_id`, sem nuvem) → abre o TaskModal para
  detalhar a tarefa nova.
- **Comandos Tauri da tarefa:** `create_requirement_card`, `update_requirement_card`,
  `update_requirement_card_status`, `set_requirement_card_projects`,
  `archive_requirement_card`/`restore_requirement_card`, e os de anexo
  (`add`/`list`/`remove`/`preview`/`download_requirement_attachment`).

## Convenções deste fork

1. **Modo single-user local.** Não há login nem pareamento de device — o app vai **direto
   pra UI**. Todo o estado sintético de auth (`LOCAL_WIRED_STATUS`, `wiredAuth*`,
   `WiredCloudBootstrap`, etc.) e os gates (`WiredLoginGate`, `CloudWorkspaceInstallGate`)
   foram **removidos**. Criar/abrir workspace é local (`createWorkspace` /
   `setWorkspaceModalOpen`).

2. **Zero código de nuvem.** Não há mais nenhum `WiredCloud*`/`Cloud*` no frontend nem
   `cloud_*` no backend. Os únicos "cloud" que sobram no Rust são `linux_cloud` /
   `cloud_init` (família de imagem / provisionamento de VM Ubuntu) — **sem relação** com a
   nuvem CLIA. Idem `ureq` (downloads do RTK, probe de VNC local, `fetch_url`): é rede local
   legítima, não portal.

## Build / run / verify

```bash
corepack pnpm install           # se node_modules quebrar, use: rm -rf node_modules && corepack pnpm install --offline
corepack pnpm dev               # vite + tauri dev (abre a janela). Requer $DISPLAY.
corepack pnpm typecheck         # tsc --noEmit
corepack pnpm test              # vitest run
corepack pnpm build:web         # tsc + vite build → dist/
corepack pnpm build             # bundle desktop completo (tauri build) — lento (LTO)
cargo check  --manifest-path src-tauri/Cargo.toml
cargo test   --manifest-path src-tauri/Cargo.toml --lib
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings   # ver "dívidas" abaixo
```

Estado verde conhecido (último check): `cargo check` 0 erros, `cargo test --lib` 199 ok,
`pnpm typecheck` 0 erros, `pnpm test` 211 ok, `pnpm build:web` ok, `tauri dev` abre a janela
e cria o SQLite local.

## Dívidas conhecidas (não bloqueiam o build)

- O **dead-code de nuvem foi removido** (structs/tabela/helpers `cloud_*` em `store.rs`,
  wrappers em `tauri.ts`, tipos `Cloud*`/`Wired*` em `types.ts`, componentes e handlers em
  `App.tsx`, chaves `flows.*` de portal no i18n, e a dep `tungstenite`). Os módulos órfãos
  `capabilities-status.*` e `task-status.*` também foram deletados.
- Restam **3 warnings de `clippy`** de dead-code **de skills** (não de nuvem):
  `WorkspaceSkillInstallInput`, `install_workspace_skill`, `skill_names` em `store.rs`.
  Inofensivos; podar quando mexer ali.

## Histórico

Fork criado a partir de `clia-wks/app/` (sem `.git` original). Commits relevantes:
`feat: clia-local — fork local-only do app (sem nuvem)` e a limpeza total de nuvem +
redesenho da Fila como kanban de tarefas locais. Veja `README.md` para o resumo de produto
e `DESIGN.md` para os tokens de UI (paleta dark-only via CSS custom properties `--clia-*`
em `src/styles.css`).
