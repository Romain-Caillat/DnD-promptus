# Promptus

A self-hosted narrative engine for tabletop RPGs, played fully remote:
the GM prepares an AI-generated campaign and runs it with an LLM co-GM;
players play from their phones.

**Status:** rewrite in progress (Rust/Axum + React + Tauri). The design
is done; the code work is ordered in `TICKETS.md`. The V1 (Next.js) is
archived in `archive/promptus-v1-nextjs.zip`.

- `CLAUDE.md` — vision, stack, structure, development environment
- `MEMORY.md` — durable decisions and invariants
- `TICKETS.md` — backlog

## Development

Requires Rust (stable), Bun and Docker. On Romain's server, all of this
lives in the PCT 105 container — see `CLAUDE.md`, *Development
environment*.

```bash
cp .env.example .env
bun install && (cd front && bun install)
bun run dev        # Postgres + Axum (:4333) + Vite (:4334)
bun run test       # Rust tests against the test database, then Vitest
bun run lint       # fmt, clippy, ESLint, tsc, knip
```

Open <http://localhost:4334>: the placeholder page says whether the
server and its database answer.

Database: `bun run db:start`, `db:stop`, `db:reset` (drops the volume).
Desktop shell: `bun run dev:tauri`; iOS simulator (macOS only):
`cd src-tauri && bunx tauri ios init` once, then `bun run dev:ios`.

License: MIT.
