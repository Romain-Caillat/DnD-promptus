# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Vision

**Promptus** is a self-hosted narrative engine for tabletop RPGs, played
**fully remote**: the game master (GM) prepares and runs the campaign,
an LLM co-GM proposes, and every player plays from their own screen —
most often a phone.

The app plans and generates the campaign (story graph, NPCs, maps,
images, video, music cues); the GM reviews and edits it; at the table
the GM drives the session, validates the co-GM's proposals and reveals
content to the players.

### Design rules (non-negotiable)

1. **The GM has the last word.** The AI proposes, the GM validates.
   Nothing reaches the players without validation.
2. **AI works in prep and live**, always behind that validation.
3. **Rules are clear.** A player always sees what they can do; the
   actions offered are derived from the campaign's rule system.
4. **No mock.** A feature is done only when it is usable end to end in
   a real session. "Shipped, never tried for real" is a status, not done.
5. **French first.** Interface and generated content in French.
6. **Phone first for players.** The player screen is designed for a
   phone; the GM screen for a laptop or tablet.

Source of these decisions: `archive/bmad/planning-artifacts/vision-v2-decisions.md`.

## Status — rewrite in progress

The V1 (Next.js 16 + Drizzle, epics E0–E9) is frozen in
`archive/promptus-v1-nextjs.zip`. It works and is the **functional
reference** for the rewrite: its tests, schema, YAML formats, prompts
and demo campaign are the specification to port. What shipped is
summarized in `archive/tickets/v1.md`.

The rewrite follows the Devotion model (same stack, same conventions).
The design is done (`archive/tickets/design.md`): the canvas boards are
the visual specification (`docs/design/README.md`), and `TICKETS.md`
orders the code work in three milestones, each ending with a real
session played.

## Tech Stack (target)

- **Desktop/Mobile shell**: Tauri 2.x (Rust) — iOS, Android, desktop
- **Frontend**: React 19 + TypeScript + Vite + shadcn/ui + Tailwind CSS v4
- **Backend**: Rust + Axum 0.8 (tokio), WebSocket for live sessions
  (replaces V1's separate Node realtime server)
- **Database**: PostgreSQL 17, sqlx 0.8 (compile-time checked queries)
- **Rules engine**: Rust, in `shared/` — the server is authoritative on
  every rule (movement, range, line of sight, attack resolution)
- **AI**: OpenRouter (LLM, image, video), behind swappable provider traits
- **Music**: YouTube (chosen, not generated), synced player-side
- **Package manager**: Bun (frontend + scripts), Cargo (Rust workspace)
- **Deployment**: Docker Compose (Axum server + PostgreSQL) on a home server

## Project Structure

```
front/          # React frontend (Vite), i18n in src/i18n (fr.json = reference)
back/           # Rust/Axum backend (API + WebSocket), sqlx migrations
shared/         # Shared Rust crate (rules engine, models)
src-tauri/      # Tauri 2.x shell (desktop/mobile app)
scripts/        # Dev helpers (test database creation)
docs/           # Reference docs (design brief, security, realtime…)
archive/        # V1 zip, retired BMad artifacts, shipped tickets per epic
```

The workspace is a skeleton (`platform/scaffold-workspace`): a
`GET /api/health` that checks the database, one migration
(`set_updated_at()` trigger function), and a French placeholder page
that calls the health endpoint.

## Development environment

The Proxmox host has **no Rust and no Docker** (and Docker must never
run there). Everything builds and runs in the LXC container **PCT 105**,
which mounts `/mnt/storage/DnD-promptus` at the same path:

```bash
pct exec 105 -- bash -lc 'cd /mnt/storage/DnD-promptus && bun run test'
```

PCT 105 is unprivileged (root inside = uid 100000 on the host). Files
created from the host (editor, Claude's Write/Edit tools) belong to
uid 0 and are **not writable from inside the container**, which then
also cannot create `target/` or `node_modules/` under them. After
writing files on the host and before running anything in PCT 105:

```bash
chown -R --from=0:0 100000:100000 /mnt/storage/DnD-promptus
```

Git runs fine on the host. Inside PCT 105 there is **no Node**: Bun
stands in for it (`bun run` shims `node`; call a Node CLI directly with
`bun --bun node_modules/.bin/<cli>`). Vitest is pinned to 4.x —
Vitest 5 + jsdom 30 fail to start their workers under Bun.

| Command | What it does |
|---------|--------------|
| `bun run dev` | Postgres (Docker), Axum on `:4333` (bacon, reloads on change), Vite on `:4334` proxying `/api` |
| `bun run test` | `cargo test --workspace` against `TEST_DATABASE_URL`, then Vitest |
| `bun run lint` | `cargo fmt --check`, clippy `-D warnings`, ESLint, `tsc -b`, knip |
| `bun run db:start` / `db:stop` / `db:reset` | Dev Postgres 17 on `127.0.0.1:5432` |
| `bun run build:tauri` | Production Tauri build (`dev:tauri`, `dev:ios` for live runs) |

First time: `cp .env.example .env`. Postgres holds two databases,
`promptus` (dev) and `promptus_test` (integration tests, created by
`scripts/create-test-db.sql` on a fresh volume). Integration tests
**fail** — they do not skip — when `TEST_DATABASE_URL` is unset, and
refuse to run when it equals `DATABASE_URL`. The compose project name
is fixed (`promptus-dev`), so every worktree shares one database.

The iOS simulator needs macOS: `tauri ios init` / `bun run dev:ios`
are run on Romain's Mac, not in PCT 105.

## Tickets and memory

- `TICKETS.md` — active backlog. Named epics, named tickets
  (`epic/verb-object`), never numbered.
- `MEMORY.md` — durable decisions, invariants, traps. **Read it first.**
  Its V1 invariants must survive the rewrite.
- `archive/tickets/<epic>.md` — what shipped, one line per ticket.
- `archive/bmad/` — the retired BMad export (brief, architecture,
  retrospective, V2 vision). Historical reference only; BMad is no
  longer used — work is tracked in `TICKETS.md` and `MEMORY.md`.

## Communication

- All conversations and documents with the user should be in **French**
  (tickets included).
- Code, code comments, commit messages and technical docs in **English**.
