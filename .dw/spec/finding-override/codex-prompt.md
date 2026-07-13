# codex — clia-local 0.2.4: override de finding bloqueante pelo dono (fim do whack-a-mole)

Worktree `/home/bruno/code/clia-local-override` (branch `feat/review-finding-override`, base 0.2.3).
Contexto: heurística de scanner de segredos SEMPRE terá falsos-positivos residuais (caso real do
dono: token fake de teste `Bearer API_test-secret` num src/*.rs e valores dev-grade no
.env.example). A resposta de produto é dar ao DONO um override consciente por finding — como o
"dismiss" do GitHub secret scanning. Commits atômicos, sem push.

## Feature

1. **Backend** (`deploy.rs`/`deploy_package.rs`/store): ação `dismiss_review_finding` por versão de
   deploy: recebe o finding (path+reason) + justificativa (texto obrigatório, ≥10 chars). Persistir
   no SQLite (novo campo/tabela junto de `blocking_findings_json` — ex. `dismissed_findings_json`
   com {path, reason, justification, dismissed_at}). Finding dismissado conta como NÃO-bloqueante
   para `approve_version` (o gate de "sem findings bloqueantes" passa a considerar só os não
   dismissados). O dismiss é POR CONTEÚDO: se uma NOVA versão do pacote reapresentar o MESMO
   finding (mesmo path relativo + mesmo reason), o dismiss anterior da stack é reaproveitado
   (herda, marcado "herdado de deploy-NNN") — senão o dono re-dismissaria a cada versão.
2. **Segurança do override**: dismissar NÃO remove o finding da UI — vira badge "aceito pelo dono"
   (estilo warning) com a justificativa visível; log/trace do dismiss; um botão "reverter".
3. **UI** (`DeployPackagesPanel.tsx`): em cada finding BLOQUEIO, botão "Aceitar risco" → modal/campo
   de justificativa → chama o comando; card mostra estado aceito; contador de bloqueios e o gate de
   Aprovar refletem na hora. Findings warning não precisam de nada.
4. **Testes**: Rust (dismiss torna aprovável; herança por conteúdo entre versões; reverter volta a
   bloquear) + vitest do que for lógica pura de UI (formatação/contagem).

## Fence e gate

Worktree only; sem push; sem binaries/. Gate: `cargo fmt --check` · `clippy --all-targets -- -D
warnings` · `cargo test` · `tsc --noEmit` · `pnpm test` · `vite build`. Auto-nota; <9 repete.
Relatório: STATUS · entrega→teste · VERIFICATION · desvios · auto-nota.
