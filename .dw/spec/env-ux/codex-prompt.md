# codex — clia-local: deixar ÓBVIO quais variáveis de ambiente estão pendentes

Worktree `/home/bruno/code/clia-local-env-ux` (branch `fix/env-pending-ux`, base 0.2.3 `36b35cd`).
Feedback literal do dono no 3º deploy: "não ficava claro quais eram as duas variáveis". O painel
"Ambiente local" mostra o contador "N variáveis pendentes" mas os campos obrigatórios não
preenchidos não se destacam num grid de ~16 inputs (o label OBRIGATÓRIA é minúsculo). Um commit,
sem push.

## Fix (em `src/DeployPackagesPanel.tsx` + `src/styles.css`, seguindo os padrões visuais do painel)

1. **Nomes no indicador**: onde mostra "N variáveis pendentes", listar os NOMES das pendentes
   (ex.: "2 pendentes: DATABASE_URL, SMTP_URL" — trunque com "+N" se passar de ~4). Mesmo texto no
   card de REVIEW do topo se for barato.
2. **Destaque visual nos campos pendentes**: input obrigatório vazio ganha estado visual claro
   (borda/glow de atenção coerente com o tema + badge "pendente"); ao preencher, o destaque some
   (estado reativo, não só no load).
3. **Agrupamento**: campos OBRIGATÓRIOS primeiro (seção própria), opcionais depois (podem ficar
   numa seção "Opcionais" colapsável se o painel já tiver padrão de collapse; senão só a ordenação
   com subtítulos).
4. **Botão Salvar**: se houver pendentes, o botão mostra o estado (ex.: "Salvar ambiente — 2
   pendentes") sem bloquear o save parcial (comportamento atual de salvar deve ser preservado).

Strings: siga o padrão de i18n existente no arquivo (se as strings do painel são literais PT,
mantenha; se via helper t(), use-o).

## Gate

`corepack pnpm exec tsc --noEmit` · `corepack pnpm test` · `corepack pnpm exec vite build` ·
teste vitest do agrupamento/pendências se houver harness de componente (senão typecheck basta,
mas tente um teste de lógica pura extraída — ex. função que particiona/formata pendentes).
Auto-nota; <9 repete. Relatório: STATUS · fix→evidência · VERIFICATION · auto-nota.
