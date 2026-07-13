# codex fix — clia-local ADE round 1 (gate: nota 5; 1 HIGH de bypass + 3 LOW)

Worktree `/home/bruno/code/clia-local-ade-fix` (HEAD `2b05957`; F1.1-F1.5 implementados — o gate
confirmou UI/detector/env sólidos e o falso-positivo do dono corrigido). O problema: o matcher
token-aware ENFRAQUECEU a proteção. Corrija G1–G4; cada um com teste. Um commit, sem push.

## G1 — HIGH: bypasses reabertos no dangerous-command (`src-tauri/src/deploy_plan.rs:1026` has_root_target_command)

O gate PROVOU (compilando os dois matchers) que o substring antigo bloqueava e o novo PASSA:
`rm -rf //` (no Linux `//` == raiz → wipe real), `$(rm -rf /)`, `` `rm -rf /` ``,
`rm -rf />/dev/null`, `rm -rf /<x` — idem `chmod -R 777 //` e `$(chmod -R 777 /)`. Causa: o
whitelist de terminadores (`*`, whitespace, `"`, `'`, `;`, `&`, `|`, EOF) omite `/`, `)`,
backtick, `>`, `<`.

**Fix — INVERTA a lógica**: após casar o prefixo (`rm -rf /`, `chmod -r 777 /` lowercase), o
comando é PERIGOSO **a menos que** o próximo char inicie um componente de path real:
`[A-Za-z0-9._~-]`. Ou seja: `rm -rf /var/...` passa; QUALQUER outra coisa depois da barra
(incluindo `/`, `)`, backtick, `>`, `<`, `$`, `*`, aspas, whitespace, EOF) bloqueia. Isso fica
estritamente ≥ a proteção do substring antigo, mantendo o falso-positivo do dono resolvido.
Testes: TODOS os bypasses listados acima bloqueiam; `rm -rf /var/lib/apt/lists/*` e
`rm -rf /tmp/build` passam; matriz antiga continua verde.

## G2 — LOW: single-quote é literal no shell (`deploy_plan.rs:994` secret_assignment_is_placeholder)

`password='$ecret123'` hoje passa como var-ref porque as aspas são trimadas ANTES da
classificação. Single-quote NÃO expande no shell → conteúdo é literal → deve BLOQUEAR.
Double-quote (`password="$VAR"`) continua placeholder. Teste dos dois lados.

## G3 — LOW: teste da fixture real mente em CI (`deploy_plan.rs:1364`)

`real_owner_lettrebox_deploy_plan_fixture_validates_when_available` retorna vazio se
`/home/bruno/wks/letrebox/...` não existir → verde vácuo fora da máquina do dono. **Fix**: copie o
`deploy-plan.json` real para fixture DO REPO (ex.: `src-tauri/tests/fixtures/` ou
`src-tauri/src/fixtures/` conforme convenção; confira que não há secret no arquivo — é plano de
agente) e faça o teste incondicional lendo a fixture embarcada; delete o teste when_available.
O teste inline `real_lettrebox_blocked_dockerfile_now_validates` (:1300) permanece.

## G4 — LOW: comentário `# NOTE=...` vira chave optional (`src-tauri/src/deploy_env.rs:267`)

Restrinja o parse de optional ao formato SEM espaço após `#` (`#KEY=`) e/ou exija que o
`.env.example` gerado seja a única fonte (formato canônico `#KEY=`). `# NOTE=set later`
(espaço após #) = comentário comum, ignorado. Teste dos dois formatos.

## Fence e gate

Iguais ao prompt original (`codex-prompt.md` nesta pasta): worktree only, sem push, sem tocar
binaries/. Gate: `cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo test`
· `corepack pnpm exec tsc --noEmit` · `corepack pnpm test` · `corepack pnpm exec vite build`.
Auto-nota 0–10; repita enquanto <9. Relatório: STATUS · G→fix→teste · VERIFICATION · auto-nota.
