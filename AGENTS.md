# CLAUDE.md — clia.local

Contexto para um agente de IA continuar este projeto. Leia isto antes de mexer no código.

## O que é

`clia.local` é a versão **100% local** do app desktop CLIA. **Sem nuvem, sem portal, sem
pareamento de device** — tudo roda na máquina (SQLite + git CLI + agentes locais).

Foi criado como **fork enxuto** de `../clia-wks/app/` (o app desktop do monorepo
`clia-wks`, que tem portal na nuvem). A camada de nuvem foi arrancada e as features que
dependiam do portal foram re-localizadas. **Não há código compartilhado vivo com o
monorepo** — é uma cópia independente.

> Regra de ouro: **não reintroduza nuvem.** Nada de HTTP para portal, sync, device login,
> websockets, analyze/opportunities. Se uma feature precisar de dados, ela vem do SQLite
> local, do git local ou de um processo de agente local.

## Stack & layout

Tauri 2 · React 19 · TypeScript · Rust. Frontend em `src/`, backend Rust em `src-tauri/`.

| Caminho | Papel |
|---------|-------|
| `src/App.tsx` | **Monolito de ~12k linhas** — quase toda a UI e o estado vivem aqui. |
| `src/tauri.ts` | Camada `api.*`: wrappers de `invoke` para cada comando Tauri. Todos passam por `invokeSafe` → retorna `{ ok: true, value } \| { ok: false, error }` (nunca lança). |
| `src/types.ts` | Tipos TS compartilhados (espelham os structs Rust). |
| `src/queue.ts` | `buildQueue()` + tipos `QueueCard`/`QueueBucket` da fila kanban. |
| `src/source/`, `src/monaco/`, `src/lsp/` | Editor de código (Monaco), LSP. |
| `src/*.test.ts(x)` | Testes Vitest (lógica pura: queue, diff, git graph, etc.). |
| `src-tauri/src/lib.rs` | ~5k linhas: registro de **todos** os `#[tauri::command]` no `generate_handler!` + `app_data_dir()`. |
| `src-tauri/src/store.rs` | ~6.7k linhas: SQLite (rusqlite, bundled). Schema + migrations + queries. |
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

Removidos do fork: **knowledge** e **skills** (saíram do `navItems`).

## Convenções deste fork (importante)

1. **Modo single-user local.** Não há login. O app inicia "pareado" via duas constantes em
   `App.tsx` (perto de `APP_VERSION`): `LOCAL_USER_ID = "local"` e `LOCAL_WIRED_STATUS`
   (status sintético `paired/connected`). O estado inicial de `wiredAuthChecked` /
   `wiredAuthSessionUnlocked` é `true`.

2. **A Queue é local.** `refreshWiredCloudBootstrap()` (em `App.tsx`) **NÃO** chama a nuvem
   — ela monta um `WiredCloudBootstrap` sintético a partir de `api.listWorkspaces()` +
   `api.listRequirementCards(wsId)`, com `current_user_id = "local"` e cada card
   `assignee_user_id = "local"`. O `QueuePanel`/`buildQueue` originais foram reaproveitados
   sem alterar a forma dos dados. Mudar status de card → `api.updateRequirementCardStatus`.
   Criar card → handler `createQueueCard()` → `api.createRequirementCard` (ID público
   reservado **localmente** por `store.rs::reserve_next_public_id`, sem nuvem).

3. **Os gates de nuvem foram removidos.** No render principal de `App.tsx` não existem mais
   os blocos `WiredLoginGate` nem `CloudWorkspaceInstallGate` — o app vai direto pra UI.
   Criar/abrir workspace é local (`createWorkspace` / `setWorkspaceModalOpen`).

4. **Chamadas de nuvem residuais são no-op seguro.** `tauri.ts` ainda exporta wrappers como
   `cloudStatus`, `syncWorkspaceToCloud`, `wiredCloudBootstrap`, etc. Os comandos
   correspondentes **não existem** no backend, então `invokeSafe` devolve `{ ok: false }`.
   Nenhuma rede é alcançada. **São código morto** — pode podar, mas não precisa.

## Build / run / verify

```bash
corepack pnpm install           # se node_modules quebrar, use: rm -rf node_modules && corepack pnpm install --offline
corepack pnpm dev               # vite + tauri dev (abre a janela). Requer $DISPLAY.
corepack pnpm typecheck         # tsc --noEmit
corepack pnpm test              # vitest run
corepack pnpm build:web         # tsc + vite build → dist/
corepack pnpm build             # bundle desktop completo (tauri build) — lento (LTO)
cargo check  --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings   # ver "dívidas" abaixo
```

Estado verde conhecido (último check): `cargo check` 0 erros, `pnpm typecheck` 0 erros,
`pnpm build:web` ok, `tauri dev` abre a janela e cria o SQLite local.

## Dívidas conhecidas (não bloqueiam build; bloqueiam `clippy -D warnings`)

- **`src-tauri/src/store.rs`**: ~8 warnings de dead-code de helpers de card de nuvem órfãos
  (`CloudMapping`, `CloudRequirementCardInput`, `normalize_cloud_requirement_status`,
  `cloud_mapping_from_row`, métodos não usados em ~`store.rs:543`). Podar quando for mexer ali.
- **`src/App.tsx` / `src/tauri.ts`**: componentes e handlers das telas removidas continuam
  no arquivo como **código morto não renderizado** (`WiredLoginGate`,
  `CloudWorkspaceInstallGate`, `KnowledgeBasePanel`, `SkillsPanel`, e os device-login
  handlers `startRequiredWiredLogin` etc.), além dos wrappers de nuvem em `tauri.ts` e dos
  branches `activeTab === "knowledge"/"skills"`. O type `Tab` ainda lista `knowledge`/`skills`.
  Limpeza opcional: remover esses blocos e apertar o union `Tab`.

## Histórico

Fork criado a partir de `clia-wks/app/` (sem `.git` original). Primeiro commit:
`feat: clia-local — fork local-only do app (sem nuvem)`. Veja `README.md` para o resumo
de produto e `DESIGN.md` para os tokens de UI (paleta dark-only, extraída de `src/styles.css`).
