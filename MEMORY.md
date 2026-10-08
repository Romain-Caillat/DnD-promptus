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
  The co-GM hears only the GM, push to talk (`copilot/listen-by-voice`):
  it never listens to the table.
- **Only the GM creates content.** Players do not author the world.
- **Rules are data.** The GM defines a rule system per campaign
  (abilities, roll formula, actions, conditions, resources, movement per
  map level, action economy, cooldowns). A rule system is a versioned
  data file the GM edits; no rule is hard-coded. The first two systems
  are drafts drawn from Romain's games (§6); the D&D 5e SRD is the
  third preset (`content/rules/srd/`, October 2026). The player UI is
  derived from the rule system.
- **No preset gets a case of its own in the engine.** What the SRD
  needed became generic, optional fields of the format (an attack bonus
  over `level`, a class's own hit points and armour class, traits,
  damage types); what the format cannot carry is written down
  (`NOT MODELLED` in the file, `docs/rules-format.md`), never approached
  in silence. The SRD 5.1 is CC-BY-4.0: its attribution stays in the
  file's header and `sources`, and its French text is our own wording.
- **A house rule the server judges is formal, and validated by the GM.**
  The co-GM proposes the formal form of a rule written in French
  (trigger, filters, effects built on the action primitives, what
  players see, cases the server replays); the GM adds it to the draft.
  Triggers are limited to what the engine already sees (an attack that
  lands or misses); anything else stays the GM's to apply.
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
- **Faction affinity (hypothesis of October 2026, to confirm with
  Romain).** A gain with a faction lowers each rival *it declares* by the
  same amount, each gauge within its own bounds; a loss moves no one
  else. Players see only factions they know of, never `diplomacy` (the
  GM's advice). It lives in the world state, moved by GM reveals.
- **Media are pre-generated** (async jobs, cached on disk), never live
  by default. Cost is estimated before each batch and checked against
  the campaign's AI budget.
- **One app, three layouts, chosen in JavaScript** (phase 6). The
  player page unfolds into columns on a computer (wide screen and a fine
  pointer, `DESKTOP_QUERY`); a tablet keeps the phone layout on the
  player side. The GM live screen switches to a rail on a coarse
  pointer of 768 px and more (`TABLET_QUERY`), or with `?ecran=tablette`
  / `?ecran=ordinateur`: an iPad in landscape is 1180 px, as wide as a
  laptop, so width alone cannot tell them apart. The layout is a media
  query read by `useMediaQuery`, not CSS hiding, so each part is mounted
  and fetched once. Panels are not copied for the tablet: they read the
  `TouchScreen` context and grow to a finger's size.


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

- **World maps are travelled, not walked (`maps/travel-hex-world`).**
  On a map in hexes the party is one token, moved only by the journey
  the GM plays portion by portion; nobody drags it (`TRAVEL_MAP`). Its
  hex, the hexes it has seen, the day and the supplies live in
  `travels` (one row per campaign and world map) and are copied onto the
  board in the same transaction, so a world map comes back as it was
  left. How a map is crossed (pace per terrain, portions, supplies,
  watches, event tables) is a travel guide next to the map
  (`content/travel/`), not a field of `Map`: the map model stays the
  grid every other part reads. The players' vote on a route advises;
  the GM chooses. A route the GM proposes is shown whole to players,
  fog included — it is a road the table is told of.

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

### A death is always the GM's

What 0 hit points does is data of the rule system (`zero_hp`):
`knocked_out` never proposes a death, `death_saves` proposes one after
the failures — hidden from players — and waits. Under any rule only the
GM's confirmation kills (`players::fate::fall`): from the fight
(`ConfirmDeath`, or `Spare` for another outcome) or from the sheets. A
dead character is never deleted: status `fallen`, its sheet and history
kept for the chronicle, a row of `character_deaths`. « One character per
player » holds for the living only (partial unique index). Last words
are written once; the choice « new character » is final and the new
draft joins at the dead one's XP (`characters.starting_xp`).

### A level's hit points are a choice, kept

Level comes from XP and is never stored. When the rules add hit points
per level, each level's die-or-average is taken once and kept
(`character_play.hit_point_gains`, rolled by the server, never
rerolled); `level_seen` is the last level the player went through. XP
taken back takes the levels' hit points with it.

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

### Spent upgrade points are stored, earned ones are derived

The points a character earned come from its XP through the rules; the
points it spent are stored by ability (`character_play.upgrades`) and
applied by the shared engine (`progression::apply_upgrades`) inside
`players::play::combatant`, so every roll, fight and screen reads the
raised score. Points left = earned − spent, never negative: the GM may
take XP back, the score keeps the point. A point is spent only outside
a live session — numbers never move mid-game.

### « Précédemment… » has one gate

Whether a session's « Précédemment… » may reach players is decided in
one place, `Session::published_previously`: the evening, the between
screen, the chronicle and the shared TV all read it there (the TV first
read the raw column; the player-route leak sweep caught it at the
phase 5 merge). A draft must never be read
from `game_sessions.previously` directly.

### Money is the rules' first resource

Loot coins and shop prices are counted in `RuleSystem::currency()`, the
first resource the rules declare (the Corsaires' `or`). Rules with no
resource have no money: loot coins and shops answer `NO_CURRENCY`, and
the GM adds one in the rules editor. Every purchase and gift moves the
purse and the bag through `players::play`, never around it; a shop's
hidden lines never leave the server until revealed.

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

### Recaps reach the table only once published

« Précédemment… » and the chronicle entry are drafts until the GM
publishes them (`game_sessions.published_at`): ending a session never
publishes, whoever wrote the text (the GM, the facts, the co-GM). The
projection reads only published text; during the launch it sends only
the sentences the GM has shown. A player text naming a front or someone
not met is flagged to the GM (`story::recap::leaks`), never blocked.

### The server's clock acts once, and only like the GM would

The schedule's clock (`schedule::tick`) claims each step of a date with
`UPDATE … WHERE step IS NULL RETURNING`: a restart or a second server
never sends a reminder twice, and after downtime only the current step
fires. Opening the lobby on its own goes through the same code as the
GM's (`session::open_locked`: lock, validated campaign, newest rules).
Tests call `tick` with a time and a campaign; only `main.rs` spawns it.

### The shared screen is a third kind of caller

A TV (or the window the GM shares on Discord) holds its own hashed
token in a cookie on `/api/tv`, behind `require_screen`; no path names
its campaign, it is the one it was paired with. It opens no GM or player
route, and the GM's session opens none of its routes — which is what
keeps the Discord window, living in the GM's browser, free of GM data.
Its whole answer is `projection::screen`, built from the players'
projection and the spectator's grid, narrowed (never widened) by the
GM's switches; every screen route is swept like the player routes.
Presence counts screens apart, never as players.

### Every AI call is counted

Each LLM, image or video call is recorded with its cost and counted
against the campaign budget; a batch that would exceed it is refused.

### House rules fire once, and a hidden one is never named

The effects of a formal house rule never trigger a house rule (one pass
per target, `action::resolve_action`); without that, « fire on the
undead burns them » would loop. A rule with `players: effect` is cut at
the projection — the players' rules page, the change list they read and
their fight log (`Event::HouseRule { shown: false }`) — while the
condition or damage it causes still reaches them as its own event.

## 4. Traps

- **vitest under Node 26** — on a Mac with Node 26, Vitest under Node
  fails every test that touches `localStorage` (Node's own experimental
  storage shadows jsdom's). Run it under Bun, as PCT 105 does:
  `bun --bun node_modules/.bin/vitest run`.
- **Persisted engine types only take defaulted fields.** A fight, its
  combatants, their conditions and learned actions are stored as JSON;
  a new required field breaks every stored fight. New fields on
  `Combatant`, `ActiveCondition`, `ActionDef` and its tags are
  `#[serde(default)]`, and what can be read through a combatant's origin
  (its traits) is read there, not copied onto the sheet.
- **Vitest under Bun on the Mac has no `localStorage`** in some suites
  (`GmTablePage.test.tsx`: « Cannot read properties of undefined
  (reading 'clear') »), independent of the code under test.

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

- **A migration that rewrites a CHECK list wins over the others.**
  `play_adjustments.kind` (and `.actor`) are `CHECK (… IN (…))` lists
  that a migration drops and re-adds to add a value: two lots built in
  parallel that each add their kind lose one of them, whichever runs
  last. When merging, the last migration must list every kind.
- **Parallel worktrees sharing one `CARGO_TARGET_DIR` build each
  other's code.** Cargo names workspace crates by their path relative to
  the workspace, and fingerprints them by mtime: another worktree's
  newer build of `promptus_shared` is taken as fresh, and the code under
  test is not yours (« could not find `house` in `rules` » on items that
  exist in your sources). Give each worktree its own hashes
  (`cargo --config 'profile.dev.package.promptus_shared.codegen-units=251'`,
  same for `promptus_back`) or its own target dir, or touch your sources
  (`find shared/src back/src -name '*.rs' -exec touch {} +`) before
  building. Likewise one shared test database breaks as soon as two
  branches carry different migrations (`VersionMissing`): give each its
  own database.
- **Keyboard shortcuts must not double a button.** A focused button
  already acts on Space and Enter: a shortcut on the same keys rolled the
  die twice. `shortcutKey` drops Space and Enter on a focused button or
  link, every key in a field, held keys and modifier chords.
- **A finger on a map is not a mouse.** iOS cancels a pointer when it
  takes the touch (`pointercancel`): a stroke left open painted on the
  next touch. Map gestures go through the pure reducer
  `features/map/gesture.ts` (tap, stroke, two-finger pan and pinch,
  cancel), and the canvas is `touch-action: none` wherever it handles
  the drag itself.
- **YouTube player must stay visible** (YouTube terms) and starts muted:
  each player taps "activate sound". Ads can desync a player; playback
  re-syncs on its own.
- **OpenRouter video is asynchronous** (`media/generate-images-and-video`):
  `POST /api/v1/videos` returns a job, polled at `/videos/{id}` until
  `completed`, the file at `unsigned_urls[0]`. Built from OpenRouter's
  docs; no live call has run yet — check the first real one.
- **OpenRouter hears audio through the chat API** (`copilot/listen-by-voice`):
  the WAV rides base64 in an `input_audio` part (`format: "wav"`) of the
  user message, sent to a model that takes audio. Built from OpenRouter's
  docs; no live call has run yet — check the first real dictation.
- **The microphone needs a secure page**: `getUserMedia` exists only over
  HTTPS or on localhost. A tablet that opens the server by its LAN address
  in plain HTTP gets no microphone (the co-GM panel says so). The browser
  records raw samples and encodes a 16 kHz WAV itself: `MediaRecorder`
  gives WebM on Chrome and MP4 on Safari, and audio models do not all
  take both.
- **Parallel branches share `promptus_test`.** sqlx refuses to migrate a
  database holding a migration version the branch does not know: a branch
  whose tests fail on « migration … was previously applied but is
  missing » is not broken, it meets another lot's migrations. Point
  `TEST_DATABASE_URL` at a database of its own; never delete rows from
  `_sqlx_migrations`.
- **A phone's video player needs `Range`**: iOS Safari plays nothing
  served whole. Media bytes are served with 206 partial answers.
- **Reminders go through Discord, not the browser** (`session/schedule-sessions`).
  Web Push on iOS only works for an app added to the home screen; the
  table already lives on Discord, so the server posts to the channel's
  webhook. Only `https://discord.com/api/webhooks/…` (and discordapp,
  ptb, canary) is accepted, or the GM could make the server call any
  address; the URL is a secret, shown back by its last four characters.
  Discord's `<t:unix:F>` puts each reader in their own time zone.
- **Vitest under Node 26 has no `localStorage`**: Node's own experimental
  global shadows jsdom's. Run the front tests with
  `bun --bun node_modules/.bin/vitest run` (as in PCT 105).
- **V1 GM pages had no authentication.** The rewrite has a GM account
  from day one (passkeys, `platform/sign-in-gm`, see §3 "GM routes");
  `/play/*` stays accountless.
- **A `CHECK (kind IN (…))` on a shared table is a merge trap.** Two
  lots that each `DROP CONSTRAINT … ADD CONSTRAINT` with their own list
  on `table_journal.kind` (or `media_assets.kind`) silently drop each
  other's values: whichever migration runs last wins. Reuse an existing
  kind when one fits (factions and goals write `note` / `item` lines with
  a `ref`), or merge the lists by hand when two lots land together.
- **Parallel lots and the test database:** `db::migrate` refuses a
  database that holds a migration the branch does not know ("previously
  applied but missing"). Lots that add migrations side by side must each
  run their tests on their own `TEST_DATABASE_URL` database.
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
  two skins, in milestone 2. Built (`engine/support-vehicle-combat`):
  a ship battle is its own table (`battles`), not an encounter; a crew
  member's id is the character's token (`pc-<uuid>`, as in a fight), so
  XP and the player's own station need no mapping; the ship's holder
  (LUMEN) is a crew member played by the GM that never holds the crew's
  turn; a boarding pauses the battle (`status = 'boarding'`) and opens
  the scene's own encounter on its deck map, whose end resumes it.
- **Which world comes next is undecided** (resume the Corsaires, the
  Brasier, or a new one): milestone 1 plays one of the two witness
  worlds and keeps the other playable in tests.
