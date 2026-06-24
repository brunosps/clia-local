# clia.local

Versão **100% local** do app desktop CLIA — sem nuvem, sem portal, sem pareamento de
device. Tudo roda na sua máquina (SQLite + git + agentes locais).

Foi criado como um **fork enxuto** do `app/` do monorepo `clia-wks`, com **toda a camada
de nuvem removida** (cloud/wired, sync, device login, analyze/opportunities) e as features
que dependiam do portal re-localizadas.

## Stack

Tauri 2 · React · TypeScript · Rust. Estado local em SQLite
(`~/.local/share/clia-local/clia-local.sqlite3`, ou `$DW_GUI_HOME`).

## Features (sidebar)

| Tab          | O que faz no clia.local                                                                                |
| ------------ | ----------------------------------------------------------------------------------------------------- |
| **Queue**    | **Kanban de tarefas locais** do workspace ativo (SQLite). Veja abaixo.                                 |
| **Code**     | Editor Monaco + árvore de arquivos + LSP. Igual ao app original.                                      |
| **Git**      | Workbench de git (stage/commit/branches/diff/stash) via git CLI nativo.                                |
| **Deploy**   | Detecção de stack, versões e deploy em máquinas locais (Winbox). Sem nuvem.                            |
| **Agents**   | Profiles + sessions; roda Codex / Claude Code / Copilot como processos locais.                        |
| **Settings** | Tema, cor de destaque, idioma, tamanho de fonte, RTK por profile.                                     |

Removidos em relação ao app original: tabs **Knowledge** e **Skills**, e todo o
subsistema de nuvem (login WIRED, install de workspaces da nuvem, sync push/pull,
analyze/opportunities).

## A Fila de tarefas (kanban)

Um quadro **kanban** por **workspace/projeto**, pensado como um to-do com a opção de rodar
no agente:

- **4 colunas** (A fazer / Fazendo / Validando / Feito) com **drag-and-drop**; mover é
  sempre humano, e dá pra **arquivar** uma tarefa.
- Escopado ao **workspace ativo**, com **filtro por projeto**.
- **Nova tarefa** abre um **modal** onde você define título, descrição, **checklist** de
  subtarefas, prioridade (Alta/Média/Baixa), **projeto(s)** (≥1), **anexos** e um **prompt**.
- **Executar com agente:** o modal monta o prompt (título + descrição + checklist + prompt +
  anexos), deixa escolher o agente (quando há mais de um) e mostra o **streaming inline**. O
  status só muda quando **você** move o card.
- **Anexos** são copiados para dentro do workspace (em `.dw/gui/attachments/`), fora dos
  repositórios de projeto, então não sujam o git.

Persistência: reaproveita a tabela `requirement_cards` (+ colunas `priority`,
`checklist_json`, `agent_prompt`) e `requirement_attachments`. IDs públicos gerados
localmente.

## O que mudou no fork (resumo técnico)

- **Backend:** nenhum código de nuvem. As structs/tabela/helpers `cloud_*` em `store.rs`
  foram removidos, junto com a dependência `tungstenite`. Comandos novos de tarefa:
  `update_requirement_card` e `set_requirement_card_projects` (anexos já existiam).
  `cargo check`/`cargo test --lib` verdes.
- **Frontend:** `App.tsx` vai direto pra UI (sem login/pareamento). A Fila foi reescrita
  como kanban (`QueuePanel` + `TaskModal`), alimentada por `loadWorkspaceTasks()` →
  `list_requirement_cards`. Tipos `Cloud*`/`Wired*`, wrappers de nuvem em `tauri.ts`, gates,
  painéis de Knowledge/Skills e chaves `flows.*` de portal no i18n foram removidos. Módulos
  órfãos `capabilities-status.*` e `task-status.*` deletados.

## Desenvolvimento

```bash
corepack pnpm install            # já vendorizado via store local; use --offline se preciso
corepack pnpm dev                # vite + tauri dev (abre a janela)
corepack pnpm typecheck          # tsc --noEmit
corepack pnpm test               # vitest run
corepack pnpm build:web          # tsc + vite build → dist/
corepack pnpm build              # bundle desktop completo (tauri build)
cargo check --manifest-path src-tauri/Cargo.toml
cargo test  --manifest-path src-tauri/Cargo.toml --lib
```

## Dívidas conhecidas (não bloqueiam o build)

- `src-tauri/src/store.rs` tem 3 warnings de `clippy` de dead-code **de skills**
  (`WorkspaceSkillInstallInput`, `install_workspace_skill`, `skill_names`). Inofensivos;
  podar quando conveniente. (O dead-code de nuvem já foi todo removido.)
