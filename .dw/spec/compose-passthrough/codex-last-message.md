**STATUS**
Concluído no worktree, sem push. Worktree limpo.

Commits locais:
- `6845e4c fix: passthrough detected project compose`
- `c305695 fix: clarify deploy compose and env examples`

**P → fix → teste**
- P1: detector agora grava `compose_path` com prioridade `compose.deploy.*` > `docker-compose.deploy.*` > root compose > prod > dev. Propagado para project-context.
- P2: `custom_compose` + `compose_path` + plano sem `artifacts.compose` usa passthrough do source copiado via `.dw-compose-file`; não escreve compose gerado na raiz. Scripts core usam `docker compose --env-file ./.env -f <source-compose> -p <project>` com `--env-file` condicional.
- P3: validação aceita `artifacts.compose: null` nesse caso e adiciona warning auditável. Detector env keys ficam opcionais também no passthrough. Manifest/UI mostram `source_passthrough` vs `agent_artifact` vs `generated`.
- P4: env obrigatório ganhou helper, botão `Usar exemplo`, e resultado pós-save informando quantas obrigatórias seguem vazias.

**VERIFICATION**
Passou:
- `cargo fmt --check --manifest-path src-tauri/Cargo.toml`
- `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`
- `cargo test --manifest-path src-tauri/Cargo.toml` → `232 passed`
- `corepack pnpm typecheck`
- `corepack pnpm test` → `227 passed`
- `corepack pnpm exec vite build`

`docker compose config` real da fixture confirmou resolução relativa ao arquivo compose source:

```yaml
services:
  app:
    build:
      context: /tmp/clia-compose-fixture-NSzzz2/projects/lettrebox/source
      dockerfile: Dockerfile.prod
    volumes:
      - type: bind
        source: /tmp/clia-compose-fixture-NSzzz2/projects/lettrebox/source/config/x.json
        target: /app/config/x.json
        read_only: true
  mailhog:
    image: mailhog/mailhog:latest
  postgres:
    image: postgres:16-alpine
  stalwart:
    image: stalwartlabs/mail-server:latest
```

Observação: Compose v2 normaliza o bind mount para objeto `type/source/target`, mas o `source` resolve corretamente para `projects/<slug>/source`.

**auto-nota**
9.4/10. Cobriu o bug real no nível de `create_package`, scripts, validação, contexto, UI e teste empírico com Docker Compose real.