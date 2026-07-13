# codex fix — 0.2.4 round 2 CURTO (re-gate: 8.0; núcleo aprovado, revogação com 2 furos)

Worktree `/home/bruno/code/clia-local-override` (HEAD `f3f0ee5`). A promessa central está entregue
(identidade por conteúdo, todas ocorrências, auditoria). Feche X1–X5; um commit; sem push.

## X1 — MEDIUM: restore vira tombstone de STACK (não por-versão)

`inherited_dismissed_findings` (deploy_package.rs:503-550) para no primeiro carrier (newest-first)
e carriers herdados não têm audit → restore feito numa versão mais ANTIGA é ignorado e a herança
re-vive. **Fix**: decisão por LINHA DO TEMPO da stack inteira — coletar, para a identidade, o
evento mais recente entre TODOS os audits das versões da stack (dismiss vs restore); herda somente
se o último evento é dismiss. Guard de status no restore igual ao do dismiss. Teste: dismiss v1 →
v2 herda → restore em v1 → v3 NÃO herda; novo dismiss em v3 → v4 herda.

## X2 — MEDIUM: linhas idênticas duplicadas = identidade ambígua e irrevogável

`select_blocking_finding` (:579-599) baila "ambiguous" para dismiss E restore quando duas
ocorrências têm a mesma identidade (mesma linha trimada 2x no arquivo). **Fix**: adicionar
discriminador ordinal (n-ésima ocorrência da mesma identidade no arquivo) APENAS para seleção/UI;
a herança continua por conteúdo (duplicata idêntica herda junto — correto e documentado), e
dismiss/restore aplicam-se a TODAS as ocorrências da mesma identidade (semântica de lote, refletida
na UI: "2 ocorrências"). Teste: linha duplicada → dismiss cobre ambas → restore cobre ambas.

## X3 — LOW: fallback path+reason só para findings SEM conteúdo por natureza

`review_identities_match` (:697-709) degrada para path+reason se qualquer lado não tem sha —
registro legado do round anterior reviveria matching fraco para secret-content. **Fix**: fallback
path+reason APENAS quando o finding é de classe sem conteúdo (ex.: estratégia não suportada);
para secret-content, sem sha = sem match (não herda). Migração: dismissals antigos sem sha não
herdam mais (aceitável — re-dismiss com identidade forte).

## X4 — LOW: finding de estratégia com path relativo

`unsupported deploy strategy` (:204-212) usa project.path ABSOLUTO — relativizar com a mesma régua
dos findings de conteúdo (raiz do workspace/projeto), senão a herança por path quebra entre
máquinas/moves.

## X5 — LOW: transação IMMEDIATE + busy_timeout

`update_deploy_version_review_json` (store.rs): `transaction_with_behavior(Immediate)` + `PRAGMA
busy_timeout` (padrão do workspace_reset). Teste de unidade se viável, senão asserção da pragma.

## Gate

`cargo fmt --check` · `clippy -D warnings` · `cargo test` · `tsc --noEmit` · `pnpm test` ·
`vite build`. Auto-nota; <9 repete. Relatório: STATUS · X→fix→teste · VERIFICATION · auto-nota.
