# DESIGN.md — clia.local

> Gerado por `/dw-analyze-project` a partir dos valores reais em `src/styles.css` em 2026-05-25. Cure antes de tratar como canon.
>
> **Estado dos tokens:** o projeto ainda **não tem tokens nomeados** (sem Tailwind theme, sem CSS custom properties, sem design system lib). A paleta abaixo é o uso *de fato*, extraído dos literais hex repetidos na folha única `src/styles.css`. Recomendação: promover esses valores a CSS custom properties (`:root { --bg: … }`) para que esta autoridade tenha uma fonte única, em vez de hex espalhado.

Autoridade de design para a grounding question 1 do `dw-ui-discipline` ("de onde vêm as decisões de design?"). Use isto antes de inventar cores, fontes ou spacing num redesign ou feature de UI.

## Princípio

Superfície compacta estilo **developer tool**, dark-only. Densa em informação, sem cromo de marketing. Bordas finas + raios pequenos delimitam painéis; cor é usada com parcimônia para status, não decoração.

## Tema

Dark-only — `color-scheme: dark` fixo em `:root`. Não há tema claro.

## Paleta (valores de fato)

### Superfícies (fundo, do mais escuro ao mais claro)

| Uso | Hex |
|-----|-----|
| App / body / botão primário-texto | `#111318` |
| Terminal / inputs ativos / preview body | `#10141d` |
| Terminal output (mais escuro) | `#0c1018` · `#070a10` (pre code) |
| Sidebar / topbar | `#151923` |
| Painel / card / modal | `#171d28` · `#151b25` (kanban col) |
| Hover / estado selecionado | `#202633` · `#182132` |

### Texto

| Uso | Hex |
|-----|-----|
| Primário | `#d9e2ef` |
| Forte / heading | `#f5f7fb` · `#f4f7fb` |
| Secundário / corpo | `#c9d7e8` |
| Muted / label / meta | `#93a4b8` · `#a9b7c8` |

### Bordas

| Uso | Hex |
|-----|-----|
| Sutil (divisórias) | `#2a303b` · `#2f3846` |
| Padrão (controles) | `#3a4656` |
| Hover / ênfase | `#5a6f89` |

### Acentos & status

| Uso | Hex |
|-----|-----|
| Azul (ativo / foco / seleção) | `#6ea8fe` · `#7cc4ff` (focus ring) · `#8ebcff` (links) · `#d8e8ff` (botão primário bg) |
| Verde (sucesso / ready / IDs) | `#a9e0c6` · `#4f765f` (borda) · `#14231d` (bg complete) |
| Âmbar (pending / submitted) | `#8a7440` (borda) · `#f5d28b` · `#f5d28b` |
| Vermelho (erro / failed / stale) | `#68454a` (borda) · `#ffd9de` (texto) · `#2a1418` (bg) |

> Convenção de status (pills/badges): verde = passed/ready/indexed, âmbar = submitted/pending, vermelho = failed/stale/blocked.

## Tipografia

| Papel | Família |
|-------|---------|
| UI (sans) | `Inter, ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif` |
| Código / terminal / diff / IDs | `"JetBrains Mono", "SFMono-Regular", Consolas, monospace` |

Renderização: `font-synthesis: none`, `text-rendering: optimizeLegibility`, `-webkit-font-smoothing: antialiased`.

Escala observada: `h1` 28px (welcome até `clamp(34px,7vw,68px)`), `h2` 16px (surface) / 22–24px (modal), corpo 13–18px, meta/label 12px uppercase, código 12–13px. Line-height ~1.2 em headings, ~1.45 em corpo, 1.5 no terminal.

## Forma & spacing

- **Raio:** `6px` padrão (botões, inputs, painéis, cards, modais); `999px` para pills/chips; `3–4px` para code inline/checkbox.
- **Altura de controle:** `min-height: 38px` (inputs, botão primário), `36px` (secundário/icon-button), `54px` (nav/source rows).
- **Gap de grid/flex:** `8–14px` típico; `padding` de painel `12–22px`.
- **Foco:** `outline: 2px solid #7cc4ff; outline-offset: 2px` em inputs/botões.
- **Layout:** `app-shell` vira grid `76px | 1fr` ≥900px (sidebar lateral) e empilha em mobile (sidebar fixa no rodapé). Kanban: `repeat(9, minmax(190–220px, 1fr))` com scroll-x + pan.

## Componentes (padrões observados)

- **Botões:** `.primary-button` (bg claro `#d8e8ff`, texto escuro, bold) vs `.secondary-button` (bg `#202633`, borda `#3a4656`). `.icon-button` 36×36 quadrado.
- **Inputs:** bg `#10141d`, borda `#3a4656`, raio 6px; labels em `<span>` 12px uppercase muted acima do controle.
- **Cards/pills:** bordas finas; pills com borda `999px` e cor por status.
- **Modais:** backdrop `rgb(0 0 0 / 0.62)`, painel `#171d28` centralizado, larguras `min(620–1180px, 100%)`.
- **Markdown preview & terminal:** monospace, fundos mais escuros, `white-space: pre-wrap`.

## Acessibilidade

- Contraste: texto primário `#d9e2ef` sobre `#111318` é forte; ao adicionar cor, mire WCAG AA (≥4.5:1 para texto normal). Cuidado com muted `#93a4b8` em fundos claros de painel.
- Foco sempre visível (ring `#7cc4ff`) — não remover.
- Labels de nav usam técnica de clip visually-hidden (`.nav-item span`) — manter para leitores de tela.

## Ao estender

1. Reutilize os hex acima; não introduza tons novos sem motivo de status.
2. Idealmente, **promova** estes valores a CSS custom properties e referencie por nome.
3. Mantenha o caráter compacto/denso (dev tool), não marketing.
4. Novo componente segue: raio 6px, borda fina, bg de painel `#171d28`, cor só para status/foco.
