# codex fix — 0.2.4 round 1 (gate: verify 9.5, adversarial 4.5 — REPROVADO)

Worktree `/home/bruno/code/clia-local-override` (HEAD `606eb23`). O enforcement base é sólido, mas
a promessa central foi entregue errada — e os dois defeitos se cancelam por acidente, então TÊM que
ser corrigidos JUNTOS. REGRA (repetida do histórico): PROIBIDO fixture/teste com dado que o código
de produção não produz — o teste `dismissed_review_finding_is_inherited_by_content` passou com path
fabricado e nome mentiroso. Um a dois commits, sem push.

## W1 — HIGH×2 (juntos): identidade do dismiss = path RELATIVO + marcador + HASH da ocorrência; scanner reporta TODAS as ocorrências

Hoje: identidade é (path ABSOLUTO com label da versão, reason-classe). Consequências provadas:
herança nunca casa entre versões (deploy-001 ≠ deploy-002 no path) E, se relativizassem só o path,
um token REAL futuro no mesmo arquivo herdaria o dismiss do falso-positivo (mesma classe de reason).
**Fix conjunto**:
- `scan_secret_content`/`scan_package_review_files`: reportar **todas as ocorrências bloqueantes**
  por arquivo (o `SecretMarkerOccurrence.index` já existe e é descartado; `next_secret_marker` já
  aceita offset) — um finding por ocorrência.
- Finding ganha identidade estável: **path RELATIVO** (à raiz do pacote — a UI já tem
  `deployFindingPathLabel` que relativiza pra exibir; use a mesma régua no dado, não só na view) +
  marcador + **sha256 da linha ofensora normalizada** (trim). Persistir também nº da linha p/ UI.
- `dismissed_matches`/herança casam pela identidade completa: conteúdo diferente = NÃO herda,
  volta a bloquear.
- Testes DE FLUXO REAL: criar pacote v1 (create_package de verdade) com falso-positivo → dismiss →
  criar v2 idêntica → herda; v3 com token REAL adicional no mesmo arquivo → o novo finding BLOQUEIA
  (não herdado) e o antigo continua dismissado; renomear/alterar a linha dismissada → não herda.

## W2 — MEDIUM: restore durável (tombstone)

`restore_review_finding` só limpa a versão atual; a próxima re-herda da antiga. **Fix**: restore
grava revogação (tombstone) que a herança respeita — identidade revogada NUNCA re-herda (até novo
dismiss explícito). `inherited_from_label` deve apontar a ORIGEM do dismiss (primeiro da cadeia),
não o elo intermediário. Teste: dismiss v1 → restore v3 → v4 BLOQUEIA.

## W3 — MEDIUM: auditoria append-only

`eprintln!` não é trilha. **Fix**: histórico append-only persistido (tabela `deploy_review_events`
ou array `review_audit_json` na versão/stack): {ação dismiss|restore, identidade, justificativa,
timestamp}. Dismiss duplicado NÃO sobrescreve o registro anterior (appenda); restore NÃO apaga o
histórico. UI: exibir histórico no detalhe do finding se barato (senão só persistir agora).

## W4 — LOW: validações

Restore de não-dismissado → erro claro (não Ok silencioso); dismiss checa status da versão (só
pending/review_required); read-modify-write do JSON numa transação/lock para janelas concorrentes.

## W5 — LOW: guard de versão cobre Cargo.toml

`appVersion.test.ts` compara só package.json↔tauri.conf.json — incluir `src-tauri/Cargo.toml`.

## Gate

`cargo fmt --check` · `clippy -D warnings` · `cargo test` · `tsc --noEmit` · `pnpm test` ·
`vite build`. Auto-nota; <9 repete. Relatório: STATUS · W→fix→teste (fluxo real!) · VERIFICATION ·
auto-nota.
