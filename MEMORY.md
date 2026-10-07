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
- **Fully remote first.** Every player on their own device (confirmed by the Corsaires game, §6). A table +
  TV mode comes later. Voice goes through an external tool (Discord…).
- **Only the GM creates content.** Players do not author the world.
- **Rules are data.** The GM defines a rule system per campaign
  (abilities, roll formula, actions, conditions, resources, movement per
  map level, action economy, cooldowns). A rule system is a versioned
  data file the GM edits; no rule is hard-coded. The first two systems
  are drafts drawn from Romain's games (§6); the D&D 5e SRD comes later
  as a third. The player UI is derived from the rule system.
- **Story is a graph, not a script.** Bible → fronts (threats with a
  4–6 step clock) → nodes (scenes/places) linked by clues; every key
  revelation is reachable through ≥ 3 clues in different nodes. Stable
  ids and short blocks so the LLM context stays small.
- **Maps: data and backdrop are separate.** Grid, walls, obstacles,
  positions and fog are data — the only reference for rules. A
  generated image is only a background; the app draws the grid over it.
  Three levels (revised in the October 2026 design pass): world (hex,
  ~10 km, day), place (square, ~5 m), encounter (square, 1.5 m, 6 s
  turn). The V1 "region in 500 m hexes" level is replaced by places on
  a square grid.
- **Table questions settled by the design pass (October 2026).** The
  shared screen is both: a real TV paired with a code, or the same page
  shared in a Discord window. Characters are created before the first
  session and validated by the GM cold, not in a session zero. Between
  sessions a player levels up, reads the recap and chronicle, checks
  their sheet and gives availability. A character's death goes through
  death saves the GM confirms, last words, then the player watches,
  creates a new character or waits for a hook. Spectators stay (a friend
  watching, a dead character's player).
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
- **A character is a description, drawn by the server only.** A
  `CharacterLook` (pieces of a pack + colours, `shared/src/sprite/`) is
  rendered in Rust to a PNG (`GET /api/sprites/render.png`); every
  screen shows that image enlarged pixelated, so the phone, the GM
  screen and the TV cannot drift apart. No second renderer in TS. The
  outline is drawn only on empty cells around the silhouette, so it
  never covers the face; condition effects are client-side overlays
  chosen by the rule system (`conditions[].visual`).
- **Characters turn and walk from server frames
  (characters/walk-in-four-directions).** Every pack piece draws `east`,
  `south` and `north` (an empty list is a choice, a missing one is
  refused); `west` mirrors `east`. Motion is four server-drawn frames per
  direction (`sheet.png`: rest, breath, two steps), made by moving cells
  by depth; the client only picks a frame and moves the whole image
  (walk along the path, lunge, hit blink). Attack and hit are whole-sprite
  motions, never per-piece art.
- **Every visual players see is pixel art**: sprites, items, maps, and
  also scene illustrations and NPC portraits (decided 4 October 2026;
  the ink-engraving images of the Corsaires are not reused). Each world
  keeps its own palette and mood inside that one style.
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
- **The player phone has one source of truth:** the playable prototype
  (canvas board "Jouer · la soirée de Marc"). The player screens and the
  storyboard are that same phone frozen on a moment. Every screen has a
  status banner, one subject and one main action; four tabs: Game, Map,
  Character, Journal. A free-text "Other…" option lets the player
  propose any idea to the GM, in a scene or in combat. The combat turn
  keeps the original layout: turn order, map centred on the target,
  hearts and gems, a fanned hand of skill cards, the arcade cluster.
- **The shared screen (TV) follows the same evening** from one
  component (canvas "Composant — écran TV"), driven by the moment and
  the player's actions; the playable board shows it next to the phone.
  One focal point at a time (story, die, map or loot), readable from a
  couch, a one-line feed at the bottom, nothing secret.
- **States show twice:** a pixel effect on the character (poison
  bubbles and tint, stun stars, sleep Z, invisible ghost, flames,
  chains, sweat drop, halo) and a square badge in the UI with turns
  left — black for banes, ivory for boons. An invisible player is not
  drawn on the TV; their own phone shows them as a ghost.
- **Notifications come in three sizes:** phone toasts that drop and
  stack (ivory good news, black bad news, outlined information), full
  TV moments for big events (level up, loot, skill unlocked, the card
  flips and its material shows its rarity), and one line per event in
  the GM journal.
- **Maps have three scales, all gridded:** world in hexes (~10 km),
  place in squares (~5 m), encounter in squares (1.5 m). The grid
  carries the rules everywhere.
- **Maps are drawn in three-quarter view** (Zelda / Stardew / RPG
  Maker): the GM or the LLM only places wall cells; autotiling draws the
  wall top one tile up and the front face when the cell below is open.
  Later rows paint over earlier ones, so a token north of a wall goes
  behind it, and that wall top turns see-through (x-ray) so the token
  stays visible; rules stay on the logical grid. One tileset per decor
  (16 tiles per material, dual-grid), hand-drawn or AI-generated once.
  Top-down and isometric were considered and set aside.
- **Outdoor maps use the same engine:** terrains blended with noise
  (asphalt, grass, sand, rock…), relief drawn like walls (cliff top one
  tile up, strata face; a unit up there is drawn higher and the rules
  know its height), props as pixel decor on the grid that also give
  cover, and an ambience setting (time of day, weather, light sources).
  Maps are in **muted colour**, the one place colour goes beyond stats,
  characters, items and dice; the UI on top stays black and white.
  Prototype renderer: `docs/design/outdoor/`.
- **An imported image is decor only.** Doors, chests, traps, secret
  passages and lights are objects on the grid, on layers the GM
  reveals; walls are traced on top (or proposed by the AI, then
  validated by the GM).
- **Themes are packs.** A theme (fantasy, zombies, space…) swaps
  tilesets, sprite parts, item art and the names of the six stats; the
  UI system, the grid and the rules engine stay the same.
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

A token's last move (`trail`) goes through the same projection: cut to
the cells the player sees, like an opponent's `Moved` event — a walk
through the fog must not draw where it came from. Screens replay only a
move whose `moves` count they have not shown yet, so a refetch never
walks a token twice.

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

### The sheet and the play state are two things

`characters.sheet` is what the player wrote and the GM reviewed; the
review diffs it against `reviewed_sheet`. What moves at the table (XP,
hit points lost, resources, the bag) lives in `character_play`, held by
the server, changed only through `players::play` (GM gestures, the
player's equip) under the campaign lock, each change logged in
`play_adjustments`. Anything the rules derive (max HP, level, AC, cards)
is computed through the engine, never stored. Writing play state into
`sheet` would make every adjustment show up as a player edit in the
review.

### Player tokens are hashed

Players join with an invite link, a nickname and a free character —
no account. The secret token lives on the device; the server stores
only its hash.

### GM routes sit behind the GM guard

Every GM route is mounted on the GM router in `back/src/app.rs`, behind
`require_gm` (no valid session cookie: 401), and is listed in
`back/tests/gm_routes_test.rs`, which sweeps them all. A resource is
checked with `owned_by` (`back/src/auth/guard.rs`): another GM's row
answers 404, exactly like a missing one — never 403, which would
confirm it exists. The first GM is created with the setup code from
the server log, every other one through an invitation: registration is
never open to whoever reaches the server first.

### Every AI call is counted

Each LLM, image or video call is recorded with its cost and counted
against the campaign budget; a batch that would exceed it is refused.

## 4. Traps

- **Design canvas: a board's CSS leaks into the components it imports.**
  A board class named `.fr` reshaped the item slot frame and broke the
  pixel sprites. Prefix every board's classes and keyframes
  (`docs/design/scope.py`); keep generic names only for runtime state
  modifiers, always combined with a prefixed class. Template holes are
  `{{name}}`: inside a Python f-string they must be written `{{{{name}}}}`.
- **Design canvas: the same leak hits a board's own modifiers.** The
  combat button carried `big`, which is also the damage-number class
  (thick black stroke): its label became unreadable. A modifier name
  must not already be a style of the same board.
- **Design canvas: boards must survive reduced motion.** Romain's
  Windows asks for reduced motion, and `@media (prefers-reduced-motion)`
  turns animations off: an element whose resting style is the animation's
  loud frame then shows that frame forever (fog dots became white
  squares). Give every animated element a calm resting style.
- **Design canvas: a map inside a phone needs `isolation: isolate`.**
  The `Plan` cells use z-indexes up to 900; without a stacking context
  on the wrapper they paint over the board's own overlays (banners).
- **Design canvas: element selectors leak into imported components too.**
  `.req .hd div{display:flex}` reached the divs inside an imported
  `Sprite` and pushed the pixel character over the buttons. Scoping only
  renames classes: write element selectors as children (`.req .hd > div`).
- **Checking the canvas in Chrome:** the window must stay in the
  foreground, or the page is reported hidden and never renders. Zoom
  and shortcuts do not reach the canvas through the extension: open a
  board large by setting `launch` to focused (`docs/design/README.md`).
- **Design canvas: a grid or flex box splits mixed text into items.**
  `<b>{{a}}/{{b}}</b>` with `display:grid` puts each text node on its own
  row (the clock showed 3 / 6 stacked). Wrap mixed text in one `<span>`.

- **A `CARGO_TARGET_DIR` shared between worktrees builds the wrong
  crate.** Workspace crates get the same artifact names in every
  worktree, and freshness is checked by mtime: a `promptus_shared` built
  by another worktree looks up to date, and the build fails on items
  that exist in your sources. Touch your sources (`find shared/src
  back/src -name '*.rs' -exec touch {} +`) or use your own target dir.
- **YouTube player must stay visible** (YouTube terms) and starts muted:
  each player taps "activate sound". Ads can desync a player; playback
  re-syncs on its own.
- **OpenRouter video is asynchronous** (`media/generate-images-and-video`):
  `POST /api/v1/videos` returns a job, polled at `/videos/{id}` until
  `completed`, the file at `unsigned_urls[0]`. Built from OpenRouter's
  docs; no live call has run yet — check the first real one.
- **A phone's video player needs `Range`**: iOS Safari plays nothing
  served whole. Media bytes are served with 206 partial answers.
- **V1 GM pages had no authentication.** The rewrite has a GM account
  from day one (passkeys, `platform/sign-in-gm`, see §3 "GM routes");
  `/play/*` stays accountless.
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

## 6. What Romain's real games taught (before Promptus)

Romain ran two games by hand before this rewrite. Their prep lives in
`dnd-save/` (texts versioned; the PNG images, 57 MB, stay out of git):
read it before designing content formats or rules.

- **Corsaires de la Couronne** (played 16 May 2026, act 1 of 4): a
  pirate one-shot, **six players, fully remote**. Afterwards: the GM
  struggled to keep the story continuous, to entertain six players at
  once and to improvise; players found the rules unclear and not applied
  consistently through act 1; they lacked information they needed for
  act 2. These four pains are what milestone 1 is judged on.
- **Le Brasier** (prepared 7 June 2026, not played): a space campaign
  with a well-built world (four factions, affinity, an AI crew member
  played by the GM). Its ship/crew combat was too messy and act 1 was
  under-prepared: rules must be testable before the table sees them.
- **The table is six players**, not the three of the design boards.
  Every GM screen, TV layout and turn order must hold six players
  against six adversaries.
- **Romain's own system is not D&D 5e.** The "Corsaires" system: six
  stats, mod = (score − 10) / 2, AC = 10 + DEX mod; checks d20 + mod
  against 5 / 10 / 15 / 20 with four outcomes (natural 1, fail, success,
  natural 20); 2 free actions per turn; class attacks with fixed damage,
  a precision bonus and a cooldown in turns, unlocked at levels 1, 3, 7,
  plus one free slot learned in play; +1 XP per success, every 5 XP one
  point to add to a stat, level from total XP; 10 HP; at 0 HP out of the
  fight, back for the next scene if nobody heals within 3 turns; no help
  bonus on checks. The rules model must express it as data, alongside
  the SRD.
- **His prep format is the content model to aim for.** A scene: place,
  mood and several YouTube tracks, flow, a hook with planned checks
  (stat, DC, what a 1 does), NPCs, GM key points, adversary stat blocks
  with tactics ("if three fall, the rest flee"), loot, XP, transition.
  An NPC: identity, one-line roleplay summary, traits, flaw, motivation,
  stats, disposition, what they want, what they hide, inventory,
  portrait. Images come in three kinds: places, NPC portraits, action
  scenes.
- **Rules are Romain's, not sacred.** He is happy to evolve his system;
  a change ships as a new locked version that players see before the
  next session, never mid-game.
- **The method these games produced** — four chains (rules, campaign,
  evening, feedback), each with a standard and checks that flag but
  never block — is in `docs/lecons-des-parties.md`. Both games are
  permanent test corpora: a check that misses their known defects
  (two damage models, critical info only in an optional scene, a
  missing referenced document) is broken.
- **Two witness worlds, built together.** The Corsaires and the Brasier
  are developed side by side from the first ticket: a ticket is done only
  when it works on both. They are **rewritten** in Promptus's format from
  `dnd-save/`, not converted: their defects are what the rewrite fixes,
  and none of their images is imported — every visual is redrawn in
  pixel art. Their rule systems are drafts meant to change.
- **Vehicle combat is shared.** The brig of the Corsaires (act 2: the
  Greyhound interception) and the Cure-Dent use one vehicle system with
  two skins, in milestone 2.
- **Which world comes next is undecided** (resume the Corsaires, the
  Brasier, or a new one): milestone 1 plays one of the two witness
  worlds and keeps the other playable in tests.
