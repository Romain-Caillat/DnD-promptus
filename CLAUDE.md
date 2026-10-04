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

## Project Structure (target)

```
front/          # React frontend (Vite)
back/           # Rust/Axum backend (API + WebSocket)
shared/         # Shared Rust crate (rules engine, models)
src-tauri/      # Tauri 2.x shell (desktop/mobile app)
docs/           # Reference docs (design brief, security, realtime…)
archive/        # V1 zip, retired BMad artifacts, shipped tickets per epic
```

None of these code folders exist yet; they are created by
`platform/scaffold-workspace` once the design is settled.

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
