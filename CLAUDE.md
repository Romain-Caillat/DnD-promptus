# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Vision

**Promptus** is a self-hosted, multiplayer tactical RPG in the spirit of
DnD, played **fully remote and live** — a "Baldur's Gate 3"-style game
rather than a tool for a game master. An **Author** prepares the
campaign with the AI; at the table **the game narrates**, voices the
NPCs and arbitrates the rules; the players (the Author included) play
from their own PC. The GM steps back: invisible overrides live, story
rewrites between chapters through Claude + the MCP.

- **Loop**: hub (free exploration, each player alone: NPCs, side quests,
  mini-games) → thread (group reunited: reveal, combat) → milestone.
- **Combat**: DnD 5e rules with Fire Emblem readability (grid, hit %
  preview), fought in place; tactical top view, HD-2D for the blows.
- **Look**: true HD-2D (Octopath Traveler, Triangle Strategy); sprites
  and tiles from PixelLab, portraits and key scenes from an image AI.
- **Scale**: short campaigns (4–6 chapters, levels 1–5), one chapter
  per evening, everyone connected; one-shot as the test format.

The game bible (`docs/bible.md`, French) is the vision. Its details
migrate into rules (`docs/regles/`); see **Rules, platform, tickets**
below.

### Design rules (non-negotiable)

1. **Fixed structure, live voice.** Everything that *can happen* is
   prepared before play (scenes, choices, consequences, failure
   branches). Live, the AI only narrates and voices those branches; it
   never invents one. Players pick from choices, never free text.
2. **The GM keeps an override**, invisible to players (pause, force or
   cancel a roll or action, move a character, trigger convergence), and
   rewrites the story between chapters.
3. **Rules are clear.** A player always sees what they can do and what
   each stat brings, in and out of combat; the actions offered are
   derived from the campaign's rule system. Every stat matters in and
   out of combat, and every hub offers an opportunity per stat.
4. **No mock.** A feature is done only when it is usable end to end in
   a real session. "Shipped, never tried for real" is a status, not done.
5. **French first.** Interface and generated content in French.
6. **PC first for players** (keyboard/mouse and gamepad). Phone play is
   set aside for now.

Source of these decisions: `docs/bible.md` (brainstorm of 2026-10-11),
which supersedes `archive/bmad/planning-artifacts/vision-v2-decisions.md`.

## Status — rewrite in progress

The V1 (Next.js 16 + Drizzle, epics E0–E9) is frozen in
`archive/promptus-v1-nextjs.zip`. It works and is the **functional
reference** for the rewrite: its tests, schema, YAML formats, prompts
and demo campaign are the specification to port. What shipped is
summarized in `archive/tickets/v1.md`.

The rewrite follows the Devotion model (same stack, same conventions).
About 115k lines were built for the GM-centric vision; that backlog is
frozen in `archive/tickets/backlog-avant-bible.md` and its code is the
starting point of the new tickets.

**The vision changed on 11 October 2026** (`docs/bible.md`: GM-light
game, PC first, true HD-2D in Three.js). `docs/tickets/` now orders the
work in two milestones: the Corsaires one-shot (tavern hub → thread →
dock brawl), then a short campaign. Each phase starts with its mockups
(`docs/design/v2/`); the older canvas boards (`docs/design/README.md`)
remain a style reference, not a specification. The pivot section at
the top of `MEMORY.md` lists the decisions the bible supersedes.

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
- **Deployment**: Docker Compose (Axum server + PostgreSQL) on a home server —
  one image, the server also serves the built front (`FRONT_DIR`);
  `docker-compose.prod.yml`, `docs/install.md`, `docs/backup.md`

## Project Structure

```
front/          # React frontend (Vite), i18n in src/i18n (fr.json = reference)
back/           # Rust/Axum backend (API + WebSocket), sqlx migrations
shared/         # Shared Rust crate (rules engine, models)
src-tauri/      # Tauri 2.x shell (desktop/mobile app)
mcp/            # Local MCP server (Bun, stdio) for Claude on the GM's Mac (docs/claude-mcp.md)
scripts/        # Dev helpers (test database creation)
deploy/         # Production: install, backup, restore, continuous deploy (docs/install.md)
docs/           # Rules, platform fiches, tickets, reference docs; the /docs/ site
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
| `bun run lint` | doc templates, `cargo fmt --check`, clippy `-D warnings`, ESLint, `tsc -b`, knip |
| `bun run docs` | The docs site on `:4335` (Python venv `.venv-docs`, MkDocs pinned below 2.0) |
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

## Rules, platform, tickets

Map and templates: `docs/README.md`. **Read the four indexes first**
(`docs/regles/jeu/`, `docs/regles/campagne/`, `docs/plateforme/`,
`docs/tickets/`, each `README.md`), then only the page you touch.

```
 docs/regles/jeu, docs/regles/campagne   what must be true   one page per rule
        │ Résolue par / Résout
 docs/plateforme                         how it is solved    one page per fiche
        │ Tickets / Réalise
 docs/tickets/<epic>/                    the work            one page per ticket
```

- **Every doc addition or change is as concise as possible.** A rule,
  fiche or ticket is one page, one screen: the labelled fields of its
  template, in order, each a paragraph of its own — a visual schema
  (`text` block, 15 lines at most), one to three sentences for the
  core field, at most five bullets, why in one line. No prose around
  it. `bun run check:docs` enforces it (also first step of `lint`);
  the global `sobre` skill trims the wording before a commit.
- A rule with no « Résolue par » is a platform gap. A technical limit
  never changes a rule silently: it comes back as a rule to settle.
- A decision settled in conversation becomes a page in the same commit.
- One fact lives in one place; other pages point to it by title, never
  by a number.
- The site (`mkdocs.yml`) is built into the image and served under
  `/docs/`; `bun run docs` serves it locally on `:4335`.

## Tickets and memory

- `docs/tickets/` — active backlog (`TICKETS.md` only points there).
  Named epics, named tickets (`epic/verb-object`), never numbered.
- `MEMORY.md` — code invariants and traps. Read its sections when
  touching the code they cover. Its V1 invariants must survive the
  rewrite. Game rules do not go there.
- `archive/tickets/<epic>.md` — what shipped, one line per ticket.
- `archive/bmad/` — the retired BMad export (brief, architecture,
  retrospective, V2 vision). Historical reference only; BMad is no
  longer used — work is tracked in `docs/tickets/` and `MEMORY.md`.

## Communication

- All conversations and documents with the user should be in **French**
  (tickets included).
- Code, code comments, commit messages and technical docs in **English**,
  except `docs/regles/` and `docs/plateforme/`, written in French. The
  older English reference docs (formats, install, MCP) stay as they are.
