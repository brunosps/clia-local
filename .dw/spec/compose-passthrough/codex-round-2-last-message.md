**STATUS**
Concluído em 2 commits locais, sem push. Worktree limpo.

Commits:
- `c9a0949 fix: prefill deploy env defaults`
- `4926ada fix: redesign deploy env inputs`

**Q → fix → teste**
- Q1: `.env.example` do pacote agora resolve defaults do compose `${VAR:-default}` antes do `.env.example` do projeto; defaults contam como preenchidos sem save; runtime injeta defaults + overrides salvos. Testes Rust cobrem compose estilo lettrebox, ready sem save, override vencendo default e chave sem default pendente.
- Q2: grid refeito em blocos empilhados: “Obrigatórias” compacto em 2 colunas, “Opcionais” abaixo com contador e colapso. Cenário 0 required mostra “Nenhuma variável obrigatória” sem quebrar layout.

**VERIFICATION**
- `cargo fmt --manifest-path src-tauri/Cargo.toml --check`
- `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`
- `cargo test --manifest-path src-tauri/Cargo.toml --lib` → 235 passed
- `corepack pnpm typecheck`
- `corepack pnpm test` → 227 passed
- `corepack pnpm build:web` → passou; só aviso existente de chunks grandes do Vite

Nota: o HEAD inicial real no worktree era `c2bece6`, não `c305695`.

**auto-nota:** 9.2/10.