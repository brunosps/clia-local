# clia.local

Versão **100% local** do app desktop CLIA — sem nuvem, sem portal, sem pareamento de
device. Tudo roda na sua máquina (SQLite + git + agentes locais).

Foi criado como um **fork enxuto** do `app/` do monorepo `clia-wks`, com toda a camada
de nuvem removida (`cloud.rs`, `wired.rs`, sync, device login, analyze/opportunities) e
as features que dependiam do portal re-localizadas.

## Stack

Tauri 2 · React · TypeScript · Rust. Estado local em SQLite
(`~/.local/share/clia-local/clia-local.sqlite3`, ou `$DW_GUI_HOME`).

## Features (sidebar)

| Tab          | O que faz no clia.local                                                            |
| ------------ | --------------------------------------------------------------------------------- |
| **Queue**    | Kanban local de cards de requisito (SQLite). Botão **Novo card**. IDs gerados localmente. |
| **Code**     | Editor Monaco + árvore de arquivos + LSP. Igual ao app original.                  |
| **Git**      | Workbench de git (stage/commit/branches/diff/stash) via git CLI nativo.           |
| **Deploy**   | Detecção de stack, versões e deploy em máquinas locais (Winbox). Sem nuvem.        |
| **Agents**   | Profiles + sessions; roda Codex / Claude Code / Copilot como processos locais.    |
| **Settings** | Tema, cor de destaque, idioma, tamanho de fonte, RTK por profile.                 |

Removidos em relação ao app original: tabs **Knowledge** e **Skills**, e todo o
subsistema de nuvem (login WIRED, install de workspaces da nuvem, sync push/pull,
analyze/opportunities).

## O que mudou no fork (resumo técnico)

- **Backend:** módulos `cloud` e `wired` deletados; seus `#[tauri::command]` e o registro
  no `generate_handler!` removidos. `cargo check` verde.
- **Frontend:** `App.tsx` inicia "pareado" contra o store local (`LOCAL_WIRED_STATUS`);
  os dois gates de nuvem (`WiredLoginGate`, `CloudWorkspaceInstallGate`) foram removidos;
  `refreshWiredCloudBootstrap` agora monta o bootstrap a partir de `list_workspaces` +
  `list_requirement_cards` (usuário sintético `"local"`), e o status do card vai para
  `update_requirement_card_status`.
- As chamadas de nuvem que sobraram em `tauri.ts` são inofensivas: `invokeSafe` devolve
  `{ ok: false }` (o comando não existe mais no backend), então nenhuma rede é alcançada.

## Desenvolvimento

```bash
corepack pnpm install            # já vendorizado via store local; use --offline se preciso
corepack pnpm dev                # vite + tauri dev (abre a janela)
corepack pnpm typecheck          # tsc --noEmit
corepack pnpm build:web          # tsc + vite build → dist/
corepack pnpm build              # bundle desktop completo (tauri build)
cargo check --manifest-path src-tauri/Cargo.toml
```

## Dívidas conhecidas (não bloqueiam o build)

- `src-tauri/src/store.rs` ainda tem helpers de card de nuvem órfãos (8 warnings de
  dead-code: `CloudMapping`, `CloudRequirementCardInput`, etc.). Inofensivos; podar quando
  conveniente.
- `App.tsx` ainda contém os componentes/handlers das telas removidas (gates, painéis de
  Knowledge/Skills) como código morto não-renderizado, além de wrappers de nuvem em
  `tauri.ts`. Limpeza opcional.
