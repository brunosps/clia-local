# DESIGN.md — clia.local

Autoridade de design: use isto antes de inventar cores, fontes ou spacing num redesign ou
feature de UI. A **fonte única** da paleta são as CSS custom properties `--clia-*` em
`:root` (e no override de tema) dentro de `src/styles.css`. Reutilize-as por nome.

## Princípio

Superfície compacta estilo **developer tool**, dark-only. Densa em informação, sem cromo de
marketing. Bordas finas + raios pequenos delimitam painéis; cor é usada com parcimônia para
status, não decoração.

## Temas

Dark-only — `color-scheme: dark`. Há **duas variantes** controladas por `themeMode`
(persistido em `app_state`):

- **`clia`** (padrão): fundos azul-petróleo escuros, acento verde-menta.
- **`black`**: variante quase-preta (override das mesmas custom properties).

Não há tema claro.

## Tokens (canônico — `:root` em `src/styles.css`)

### Tema `clia` (padrão)

| Token | Valor | Uso |
|-------|-------|-----|
| `--clia-bg` | `#101419` | App / body |
| `--clia-shell` | `#151b22` | Sidebar / topbar |
| `--clia-panel` | `#111923` | Painel / card / coluna |
| `--clia-panel-strong` | `#192230` | Card sobre painel / hover |
| `--clia-input` | `#0b1118` | Inputs / áreas de stream |
| `--clia-border` | `#2a3f4b` | Borda padrão |
| `--clia-border-strong` | `#514d86` | Borda de ênfase / hover |
| `--clia-text` | `#dce8f3` | Texto primário |
| `--clia-muted` | `#9cb0c3` | Label / meta / muted |
| `--clia-primary` | `#41ef6e` | Ação primária (verde) |
| `--clia-primary-hover` | `#56f0b0` | Hover primário |
| `--clia-secondary` | `#7c50e2` | Ação secundária (violeta) |
| `--clia-secondary-hover` | `#9272f3` | Hover secundário |
| `--clia-accent` | `#24d0d6` | Acento / destaque (ciano) |
| `--clia-active-bg` | `rgba(124,80,226,.13)` | Fundo de estado ativo |
| `--clia-active-bg-strong` | `rgba(65,239,110,.14)` | Ativo forte (ex.: coluna em drop) |
| `--clia-ring` | `rgba(146,114,243,.82)` | Anel de foco |

Brand: `--clia-green #41ef6e`, `--clia-mint #56f0b0`, `--clia-cyan #24d0d6`,
`--clia-blue #4289f0`, `--clia-violet #7c50e2`.

### Tema `black` (override)

`--clia-bg #050607` · `--clia-shell #090b0d` · `--clia-panel #07090b` ·
`--clia-panel-strong #0d1013` · `--clia-input #030405` · `--clia-border #222832` ·
`--clia-border-strong #42366f` · `--clia-text #e2e8ef`. Os acentos (primary/secondary/
accent) permanecem.

> Componentes mais antigos ainda podem ter hex hardcoded em `src/styles.css` (a folha é
> única e grande). Ao tocar num componente, prefira migrar o hex para o token `--clia-*`
> correspondente.

## Status (pills/badges)

Convenção: **verde** (`--clia-primary`) = ready/done/ok; **violeta/ciano** = ativo/foco;
**âmbar** = pending/submitted; **vermelho** = failed/stale/blocked. Prioridade de tarefa usa
`.priority-pill.high/.medium/.low`.

## Tipografia

| Papel | Família |
|-------|---------|
| UI (sans) | `Inter, ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif` |
| Código / terminal / diff / IDs | `"JetBrains Mono", "SFMono-Regular", Consolas, monospace` |

Renderização: `font-synthesis: none`, `text-rendering: optimizeLegibility`,
`-webkit-font-smoothing: antialiased`. Escala: `h1` ~28px (welcome até `clamp(...)`),
`h2` 16px (surface) / 22–24px (modal), corpo 13–18px, meta/label 12px uppercase, código
12–13px. Line-height ~1.2 em headings, ~1.45 em corpo.

## Forma & spacing

- **Raio:** `6px` padrão (botões, inputs, painéis); cards/colunas do board 10–12px;
  `999px` para pills/chips; `3–4px` para code inline/checkbox.
- **Altura de controle:** inputs/botão primário ~38px; secundário/icon-button ~36px.
- **Gap de grid/flex:** `8–14px` típico; `padding` de painel `10–22px`.
- **Foco:** anel via `--clia-ring`; sempre visível — não remover.
- **Layout:** `app-shell` vira grid `76px | 1fr` ≥900px (sidebar lateral) e empilha em
  mobile.
- **Boards kanban:**
  - **Fila de tarefas (aba Queue):** grid de **4 colunas** fixas
    (`.queue-panel .kanban-board { grid-template-columns: repeat(4, minmax(0,1fr)) }`),
    colunas `.kanban-column` (drop-target `.over`), cards `.kanban-card` arrastáveis.
  - **Workbench-flow (deploy/flows):** kanban horizontal mais largo, com muitas colunas e
    scroll-x/pan — classes `.kanban-shell`/`.kanban-group` (não confundir com o board da
    Fila, que é escopado sob `.queue-panel`).

## Componentes (padrões observados)

- **Botões:** `.primary-button` (bg `--clia-primary` verde, texto escuro, bold) vs
  `.secondary-button` (bg de painel, borda `--clia-border`). `.icon-button` ~36×36.
  `.ghost-button` para ações discretas em cards/modais.
- **Inputs:** bg `--clia-input`, borda `--clia-border`, raio 6px; labels em `<span>` 12px
  uppercase muted acima do controle (`.field > span`).
- **Chips:** `.chip` (pill `999px`, borda fina) com `.chip.active` (verde) e `.chip.small`
  — usados no filtro de projeto e na seleção de projetos do TaskModal.
- **Cards/pills:** bordas finas; pills com cor por status.
- **Modais:** backdrop escuro translúcido, painel `--clia-panel`/`--clia-panel-strong`
  centralizado; `.task-modal` tem `width: min(620px, 94vw)` e corpo rolável.
- **Stream de agente inline:** `.agent-inline-stream` (fundo `--clia-input`, rolável) com
  `.agent-msg` por mensagem.

## Acessibilidade

- Contraste: texto `--clia-text` sobre `--clia-bg` é forte; ao adicionar cor, mire WCAG AA
  (≥4.5:1). Cuidado com `--clia-muted` sobre fundos mais claros.
- Foco sempre visível (anel `--clia-ring`) — não remover.
- Labels de nav usam clip visually-hidden (`.nav-item span`) — manter para leitores de tela.

## Ao estender

1. Use os tokens `--clia-*` por nome; não introduza tons novos sem motivo de status.
2. Se encontrar hex hardcoded num componente que vai tocar, migre para o token equivalente.
3. Mantenha o caráter compacto/denso (dev tool), não marketing.
4. Novo componente: raio 6px, borda fina (`--clia-border`), bg de painel (`--clia-panel`),
   cor só para status/foco.
