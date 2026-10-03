# MEMORY — Durable decisions, invariants, and traps

Long-lived knowledge for Promptus. Read this before touching the
server, the player screen, or anything the LLM produces.

**What belongs here:** rules that outlive a ticket — architectural
constraints, invariants that are load-bearing, decisions that were
argued and settled, and traps that already cost someone a day.

**What does NOT belong here:** work to do (→ `TICKETS.md`), narration
of what was built (→ `archive/tickets/`), API documentation (→ code).

Rule of thumb: if it stops being true when a ticket closes, it is not
memory. If someone would re-litigate it in six months, it is.

---

## 1. Settled decisions

- **Rewrite, not migration.** V1 (Next.js) is frozen in
  `archive/promptus-v1-nextjs.zip`. Nothing is ported file by file:
  each feature is rebuilt end to end on the new stack, with the V1
  tests as its specification.
- **Same stack and conventions as Devotion**: Rust/Axum + sqlx,
  React/Vite + shadcn, Tauri, Bun. React was chosen over Vue to reuse
  Devotion's mobile hooks, item menus, i18n setup and lint rules.
- **Fully remote first.** Every player on their own device. A table +
  TV mode comes later. Voice goes through an external tool (Discord…).
- **Only the GM creates content.** Players do not author the world.
- **Rules are data.** The GM defines a rule system per campaign
  (abilities, roll formula, actions, conditions, resources, movement per
  map level). A "D&D 5e (SRD)" preset is the editable starting point.
  The player UI is derived from it.
- **Story is a graph, not a script.** Bible → fronts (threats with a
  4–6 step clock) → nodes (scenes/places) linked by clues; every key
  revelation is reachable through ≥ 3 clues in different nodes. Stable
  ids and short blocks so the LLM context stays small.
- **Maps: data and backdrop are separate.** Grid, walls, obstacles,
  positions and fog are data — the only reference for rules. A
  generated image is only a background; the app draws the grid over it.
  Three levels: campaign (hex, ~10 km, day), region (hex, ~500 m, hour),
  combat/place (square, 1.5 m, 6 s turn).
- **Media are pre-generated** (async jobs, cached on disk), never live
  by default. Cost is estimated before each batch and checked against
  the campaign's AI budget.


## 2. Visual direction (design pass, October 2026)

Settled with Romain on the design canvas (link in `TICKETS.md`, epic
`design`). The feel is a board game / video game, not a dashboard.

- **Black and white UI; colour only for numbers.** HP red `#FF4D5E`,
  ATK orange `#FF9F1C`, AC blue `#3D9BFF`, MAG violet `#B28CFF`,
  MOVE mint `#2EE6A6`, INIT yellow `#FFD60A`. Each stat also has a
  shape (diamond ATK, shield AC, hexagon MAG, arrow MOVE, round INIT),
  so colour is never the only cue.
- **Exception: characters are in colour.** Pixel-art sprites in the
  Terraria / Starbound style: side view, dark outline, top-light and
  back-shadow shading, heroes face right and enemies face left.
- **Items are pixel art in colour too** (12 × 12 cells plus outline),
  shown in square inventory slots like Terraria. An item's rarity is
  the slot frame's material, the same six as skill cards, plus 1 to 6
  diamonds. Loot on the TV: the chest shakes, a beam in the rarity's
  material rises, the item floats up.
- **Dice take the colour of the stat they roll** (attack orange,
  damage red, spell violet…); white is a plain skill roll. The faces
  keep their shading, re-tinted in that colour.
- **Game materials:** ivory playing cards on a black table (actions,
  clues, scenes) and a turn track with portraits.
- **Stat gems are small pixel grids** (7, 9 or 11 cells wide by size),
  one shape per stat, with a wave of light whose direction belongs to
  the stat: HP from the centre out at heartbeat pace, MOVE left to
  right, ATK diagonal, AC from the edges in, MAG spiral, INIT clockwise.
- **Gauges:** HP as pixel hearts (Terraria: 2 HP per heart, at most ten
  hearts, beyond that each heart holds max/10); hearts burst into pixels
  on a hit. Countable resources (spell slots, movement, ammo) as bars of
  square cells, one cell per point; spent cells whiten and fall. Threat
  clocks are rings of square cells, the next part blinking. Rounded
  progress bars and pie clocks were tried and rejected as "app-like".
- **Buttons are cards:** inner frame, Cinzel label, the stat gem when
  the action depends on one; ivory for the main action, black for the
  rest; pressing tips the card forward. On the phone in combat only, an
  arcade cluster: one big round button under the thumb for the selected
  card, item and end-turn as small round buttons around it.
- **Skill rarity has six tiers** — common, uncommon, rare, epic,
  legendary, divine — read from the card's material and from 1 to 6
  diamonds, never from a hue (hues belong to stats): ivory, double
  rule, brushed silver, black card, gold foil, holographic. The divine
  card is the only rainbow in the app.
- **Dice are all faceted** (d4 to d20, shaded faces, numbers that roll
  then settle with a flash in the die's colour). Cartoon and pixel dice were tried and
  rejected.
- **The radial square grid has two uses only:** fog of war (unknown
  cells) and the small "AI is writing" indicator. Elsewhere it was too
  much.
- **Type:** Cinzel (titles, cards), Cormorant Garamond italic (text read
  aloud), Chakra Petch (UI and numbers).
- **Motion is part of the product:** dice rolls, damage numbers, card
  deals and flips, hit shake, "your turn" slam. Every animation has a
  meaning and respects `prefers-reduced-motion`.

## 3. Load-bearing invariants (proven in V1, keep them)

Break one of these and the failure is silent, not loud.

### The player projection

The player never receives GM notes, hidden cells, the names of
unrevealed adversaries, nor their hit points. V1 built the player view
in one pure function (`projectPlayerView`) and every player route went
through it. Keep a **single projection point** on the server; never
filter on the client.

### The server decides every rule

Movement (turn, speed, walls, fog, occupied cells), range, line of
sight, and attack resolution are checked server-side. The client only
highlights what the server would accept.

### Nothing reaches players without the GM

Co-GM narration and NPC lines are editable drafts; suggestions are
applied by the GM. Ids invented by the model (clues, nodes, entities
that do not exist) are dropped before the answer is shown.

### One lock per campaign for world writes

Every write to world state or combatants takes a row lock on the
campaign (`SELECT … FOR UPDATE`) and re-reads inside the transaction.
GM and players act concurrently; without it they overwrite each other.

### Realtime carries no game data

The live channel only says "this changed" (Postgres NOTIFY relayed to
the session's sockets, plus presence). Clients refetch through the API,
so the projection is never bypassed.

### Player tokens are hashed

Players join with an invite link, a nickname and a free character —
no account. The secret token lives on the device; the server stores
only its hash.

### Every AI call is counted

Each LLM, image or video call is recorded with its cost and counted
against the campaign budget; a batch that would exceed it is refused.

## 4. Traps

- **YouTube player must stay visible** (YouTube terms) and starts muted:
  each player taps "activate sound". Ads can desync a player; playback
  re-syncs on its own.
- **OpenRouter video output format was never verified** (no network
  during V1 development). Check it before designing around it.
- **V1 GM pages had no authentication.** The rewrite needs a GM account
  from day one (passkeys, as in Devotion); `/play/*` stays tokenless.
- **YouTube player must stay visible** also applies to the phone design:
  the scene screen currently hides music behind a button — needs a
  visible mini-player before it ships.

## 5. Conventions

- **Language:** conversation and tickets in **French**. Code,
  comments, commit messages, and technical docs in **English**.
- **Naming:** epics and tickets are **named**, never numbered.
  Ticket id = `epic/verb-object`, kebab-case English.
- **Tests earn their place.** A test must fail on a plausible bug in
  observable behaviour. Never assert wiring, defaults or source text.
- **Clean cutover.** Migrate every caller; no shims or deprecated paths.
- **Additive props over forks** on shared components.
- **No UI text in code.** Every word a person reads comes from `t()`,
  French as the reference locale.
- **No mock.** No "not yet wired" button ships.
