# clia.local

**Ambiente de desenvolvimento com agentes de IA — 100% local.**

`clia.local` é um app desktop que reúne, numa única janela, tudo que você precisa para
tocar projetos com a ajuda de agentes de IA: uma **fila de tarefas** (kanban), um **editor
de código**, um **workbench de Git**, **deploy em VMs locais** e a orquestração dos seus
**agentes** (Codex, Claude Code, Copilot). Tudo roda na sua máquina — **sem nuvem, sem
conta, sem login, sem pareamento de device**. O estado vive em SQLite local, o Git usa o
binário nativo e os agentes rodam como processos locais.

---

## Por que clia.local

- **Local e privado.** Sem portal, conta ou sincronização na nuvem: seu código, tarefas e
  histórico ficam em SQLite na sua máquina. (Ações de rede explícitas — clonar repositórios,
  baixar runtimes e chamar os CLIs dos agentes — usam a internet normalmente.)
- **Abre direto no trabalho.** Sem tela de login nem conta — ao abrir, você já está na UI.
- **A pasta é o workspace.** `clia-local ~/code/meu-projeto` abre aquela pasta; os
  repositórios dentro dela viram projetos sozinhos, e os dados ficam no `.dw` da própria
  pasta. Nada de cadastrar workspace.
- **Multi-projeto por pasta.** Vários repositórios (e seus submódulos) sob a mesma pasta,
  alternando entre eles rapidamente.
- **Agentes como cidadãos de primeira classe.** Dispare um agente direto de uma tarefa e
  acompanhe o resultado — com histórico por tarefa.

## Funcionalidades

| Área           | O que faz                                                                                                   |
| -------------- | ----------------------------------------------------------------------------------------------------------- |
| **Fila**       | Kanban de tarefas locais por workspace (A fazer / Fazendo / Validando / Feito), com drag-and-drop.          |
| **Código**     | Editor Monaco com árvore de arquivos, busca, LSP, preview de Markdown e *blame* do Git.                     |
| **Git**        | Workbench completo (stage/commit, branches, diff por linha/hunk, stash, tags, rebase, submódulos).          |
| **Deploy**     | Detecção de stack, versionamento e publicação dos projetos em **VMs locais** (WinBox · Docker/QEMU).        |
| **Agentes**    | Perfis e sessões; roda Codex / Claude Code / Copilot como processos locais, com *streaming* e métricas.     |
| **Configurações** | Tema, cor de destaque, idioma, tamanho de fonte e runtime por perfil de agente.                          |

### Fila de tarefas (kanban)

Um quadro por **workspace**, com filtro por projeto, pensado como um *to-do* que pode ser
executado por um agente:

- **4 colunas** com *drag-and-drop*; mover um card é sempre uma ação humana, e dá para
  **arquivar** tarefas.
- Cada tarefa abre um **modal** com título, descrição, **checklist** de subtarefas,
  **prioridade**, **projetos** (um ou mais), **anexos** e um **prompt** para o agente.
- **Executar com agente:** o modal monta o prompt a partir da tarefa, deixa você escolher o
  agente e mostra o resultado em *streaming*.
- **Histórico do agente por tarefa:** cada execução fica registrada na própria tarefa —
  com o **prompt** e o **resultado** renderizados em Markdown — sem se perder no chat.

### Projetos & Git

- **Adicionar projeto:** clonar de um repositório Git remoto (GitHub, GitLab, Bitbucket… via
  https/ssh) — com **submódulos**, progresso ao vivo, cancelar e pedido de credencial para
  repositórios privados — ou apontar para uma **pasta local**.
- **Submódulos:** clone recursivo; cada submódulo inicializado vira um **projeto-filho** do
  workspace e ganha uma seção dedicada no Git (atualizar todos, *update*, *remote*, trocar de
  branch, indicador de *detached HEAD*).

## Stack

**Tauri 2 · React 19 · TypeScript · Rust.** O frontend vive em `src/` e o backend
(comandos, SQLite e integração com Git/agentes/VMs) em `src-tauri/`.

## Requisitos

- **Node** (via `corepack` para o `pnpm`) e a **toolchain do Rust** (`cargo`).
- Dependências de build do **Tauri** para o seu SO (no Linux, p.ex. `webkit2gtk`).
- **Git** instalado — o workbench usa o Git nativo.
- *Opcional (aba Deploy):* **Docker** (+ KVM/QEMU) no host para criar e rodar as VMs. O CLI
  do WinBox já vem **embutido** e é resolvido automaticamente (sem precisar de `WINBOX_BIN`).
- *Opcional:* os CLIs dos **agentes** que você for usar (Codex, Claude Code, Copilot).

## Começando

```bash
corepack pnpm install      # instala as dependências
corepack pnpm dev          # sobe o Vite + Tauri e abre a janela (requer $DISPLAY)
```

### Primeiro uso

1. **Abra uma pasta** — `clia-local /caminho/da/pasta`, ou *Abrir pasta…* no seletor do topo.
   A pasta é o workspace: seu nome vira o nome, e os dados vão para o `.dw` dentro dela (se
   ela for um repo git, `.dw` entra no `.git/info/exclude`, então não suja o `git status`).
   Instale o comando com `corepack pnpm launcher:install`.
2. **Os repositórios da pasta já aparecem como projetos.** Para trazer outro, *New project* →
   clonar um repositório remoto ou apontar para uma pasta local.
3. **Configure um agente** em *Configurações* (perfil + CLI do agente).
4. Crie uma tarefa na **Fila** e dispare **Executar com agente**.

### Build

```bash
corepack pnpm build:web        # build do frontend (dist/)
corepack pnpm build            # bundle desktop completo (tauri build)
```

> O `pnpm build` já roda `rtk:prepare` + `winbox:prepare` automaticamente: eles baixam os
> binários embutidos (sidecar **RTK** e o **CLI do WinBox**) e verificam o `sha256` — esses
> binários não ficam versionados. O WinBox vem da release do `winbox-gui`; o download usa o
> `gh` autenticado (cobre repositório privado). Sem acesso/rede, há *fallback* para um build
> local (`WINBOX_GUI_BIN` ou um checkout do `winbox-gui` ao lado deste repositório).

### Qualidade

```bash
corepack pnpm verify        # gate completo: typecheck + lint + test + build:web + cargo fmt/clippy/test
```

Ou cada passo isoladamente:

```bash
corepack pnpm typecheck     # tsc --noEmit
corepack pnpm lint          # eslint --max-warnings=0
corepack pnpm format:check  # prettier --check
corepack pnpm test          # vitest
cargo test --manifest-path src-tauri/Cargo.toml --lib
```

## Onde ficam os dados

Todo o estado é local, em SQLite, dividido em dois arquivos:

- **Na pasta que você abriu:** `<pasta>/.dw/clia-local.sqlite3` — tarefas, projetos, deploy
  e sessões de agente. É o dado de verdade: ele viaja com a pasta, e apagar o `.dw` apaga o
  workspace. Os anexos das tarefas ficam ao lado, por card
  (`<pasta>/.dw/gui/attachments/<card_id>/`).
- **No diretório de dados do app:** `clia-local.sqlite3` — apenas a **lista de pastas
  recentes** e as preferências de UI. Resolvido nesta ordem: `$DW_GUI_HOME` → diretório de
  dados do Tauri (`~/.local/share/dev.clia.local/`) → `~/.local/share/clia-local/` →
  `./.clia-local`.

Se a pasta aberta for um repositório Git, o `.dw` é registrado no `.git/info/exclude` desse
clone — não suja o `git status` nem o `.gitignore` versionado.

## Estrutura do repositório

```
src/            # frontend React + TypeScript (UI, editor, kanban, git, deploy, agentes)
src-tauri/      # backend Rust (comandos Tauri, SQLite, Git, agentes, VMs)
```
