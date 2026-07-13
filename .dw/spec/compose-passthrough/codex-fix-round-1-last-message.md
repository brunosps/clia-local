**STATUS**  
Concluído em 1 commit local, sem push: `648945c fix: harden deploy env prefill review`. Worktree limpo.

**Y → Fix → Teste**
- Y1: o scanner não revisa mais o `.env.example` gerado pelo ADE; `.env.example` do projeto copiado em `projects/*/source` continua sendo escaneado. Teste real com `compose.deploy.yaml` do Lettrebox cria pacote, mantém defaults e `approve_version` passa.
- Y2: defaults agora exigem marcador `#dw:default KEY project`; templates legados viram placeholders. Teste com `deploy-003.env.example` real garante zero `default_source`.
- Y3: múltiplos projetos com compose próprio não fazem passthrough silencioso; fallback para compose gerado + warning em validation/manifest. Testes 1 projeto passthrough, 2 projetos fallback.
- Y4: parser de defaults do compose agora ignora comentários, ignora `$${...}`, aceita `${VAR-default}` e não trunca default com `}` aninhado.
- Y5: runtime `.env` não injeta opcionais não salvos; `#KEY=value` em `.env.example` do projeto é comentário, não default.

**VERIFICATION**
- `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`
- `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`
- `cargo test --manifest-path src-tauri/Cargo.toml` → 239 passed
- `corepack pnpm typecheck`
- `corepack pnpm test` → 227 passed
- `corepack pnpm build:web` → `tsc && vite build` passou

**AUTO-NOTA**  
9.4/10. Cobriu os cinco itens com testes de fluxo real e fixtures reais; única ressalva é que o warning multi-compose é bloqueante do passthrough/fallback, não um finding de review que bloqueia aprovação.