# Campaign format

A campaign is **one YAML document**: the bible, the acts, the fronts,
the story graph (nodes linked by exits), the revelations and their
clues, and every entity (NPCs, adversaries, locations, items, factions,
goals). It is what the GM writes and edits by hand, what the LLM
produces, and what `campaign/rewrite-two-worlds` writes the two witness
worlds in.

- Types: `shared/src/story/model.rs` (the reference: every field is
  documented there).
- A complete example that validates with no issue:
  `content/fixtures/phare-de-kerbrume.yaml`.
- API: `POST /api/campaigns/import` and `PUT /api/campaigns/{id}/import`
  take `{ "yaml": "…" }`; `GET /api/campaigns/{id}/export` returns
  `{ "data": { "yaml": "…" } }`.

Keys are English snake_case; content (names, texts) is French.

## Ground rules

- **Ids are stable slugs**: lowercase letters, digits, `_` and `-`
  (`sc_taverne`, `pnj_morel`). They share **one namespace** across the
  whole campaign — an NPC and an item cannot both be `morel`. Prefixes
  are a convention, not a rule: `sc_` nodes, `pnj_` NPCs, `adv_`
  adversaries, `lieu_` locations, `obj_` items, `fac_` factions, `but_`
  goals, `rev_` revelations, `cl_` clues, `pj_` party, `front_` fronts.
- **Never rename an id once played**: the living world refers to ids
  (clues found, nodes visited, names revealed). Re-importing a campaign
  keeps its world.
- **Unknown keys are refused** with the line and column, so a typo is
  an import error, never a field silently lost. Enum values are
  English (`friendly`, `critical`, `combat`…).
- Every optional field may be left out; export leaves empty fields out,
  and export → import gives back exactly the same campaign.
- Everything is GM material. What players see is decided on the server
  by the player projection only (`back/src/campaigns/projection.rs`);
  the format has no "visible" flag.

## Top level

| Key | Required | What |
| --- | --- | --- |
| `format` | no (1) | Format version |
| `id`, `title` | yes | The campaign's slug and title |
| `world` | no | The universe, in a few words |
| `rules` | yes | `{ id, version }` — the rule system version the campaign plays (`engine/model-rule-system`) |
| `bible` | yes | See below |
| `party` | no | Player character slots: `id`, `name`, `player`, `concept` |
| `acts`, `fronts`, `nodes`, `revelations`, `clues` | no | The story |
| `npcs`, `adversaries`, `locations`, `items`, `factions`, `goals` | no | The entities |

**`bible`** — `pitch`, `tone`, `themes` (list), `art_direction` (the
pixel-art palette and light for every generated visual), `truths`
(list), `secrets` (list), `player_hook` (what the players are told),
`start_node` (the opening node).

## The story

**`acts`** — `id`, `title`, `summary`, `opening` and `closing` (read
aloud), `music` (tracks), `gm_notes`.

**`fronts`** — a threat with a clock: `id`, `name`, `goal`,
`description`, `steps` (4 to 6 × `{ label, description }`, the last is
the catastrophe).

**`nodes`** — a scene, carrying the scene standard of
`docs/lecons-des-parties.md` §3 chain 2:

| Key | What |
| --- | --- |
| `id`, `act`, `title` | Required |
| `optional` | `true` when the scene may be skipped |
| `summary` | For the GM, one or two sentences |
| `location` | A location id |
| `read_aloud` | Shown or read to the players on entering |
| `ambience` | `mood`, `sounds`, `music`: list of `{ mood, title, url }` — `mood` is `calm`, `exploration`, `tension`, `mystery`, `combat` or `epic`; `url` a YouTube link |
| `flow` | How the scene unfolds |
| `hook` | The event that sets it in motion |
| `checks` | Planned checks: `action`, `stat`, `difficulty`, `success`, `failure`, `natural_1`, `natural_20` |
| `npcs` | `{ npc, role }` — who is there and what for |
| `key_points` | What the GM must not forget (list) |
| `encounter` | `opponents` (`{ who, count }`, `who` an adversary or an NPC with stats), `tactics` (list), `morale` (`{ when, then }`), `on_victory`, `on_defeat` |
| `loot` | `{ item, coins, found, hidden }` — an item, coins or both; `hidden` when only found by searching |
| `xp` | `{ amount, reason }` |
| `transition` | How the scene hands over, narrated |
| `exits` | `{ to, label }` — the edges of the graph |
| `requires` | Revelation ids the players must know **entering** (what they learn here is the clues placed in this node) |
| `player_hooks` | `{ character, reason }` — a reason to act for each party member |
| `if_skipped` | The GM's note on where the critical information also is |
| `art` | The scene illustration, described for pixel art |
| `gm_notes` | Free notes |

**`revelations`** — `id`, `statement` (the conclusion, in the GM's
words), `importance`: `critical` (needed to go on) or `optional`.

**`clues`** — `id`, `revelation`, `node` (where it is placed), `text`
(what the players learn — the only part they ever see), `discovery`
(how it is found), `source` (an NPC, item or location id), `check`
(`{ stat, difficulty }`).

## The entities

**`npcs`** — Romain's NPC sheet: `id`, `name`, `title`, `age`,
`appearance`, `portrait` (for pixel art), `roleplay` (one line to play
them by), `traits` (list), `flaw`, `motivation`, `disposition`
(`friendly`, `neutral`, `hostile`), `wants`, `hides`, `stats`,
`inventory` (`{ item, quantity }`), `sells` (`{ item, price }`, a shop),
`faction`, `location`, `gm_notes`.

**`stats`** (NPCs and adversaries) — `abilities` (map of the rule
system's ability ids to scores), `armor_class`, `hit_points`, `attacks`
(`{ name, damage, notes }`, `damage` a formula like `1d6+2`). The rules
engine checks these against the campaign's rule system.

**`adversaries`** — stat-block creatures without an NPC sheet: `id`,
`name`, `description`, `art`, `stats` (required), `gm_notes`.

**`locations`** — `id`, `name`, `description` (what players see),
`parent` (the containing location), `art`, `gm_notes`.

**`items`** — `id`, `name`, `description`, `effect`, `value`, `rarity`
(`common`, `uncommon`, `rare`, `epic`, `legendary`, `divine`), `art`
(the 12 × 12 sprite), `gm_notes`.

**`factions`** — `id`, `name`, `description`, `diplomacy`, `affinity`
(`{ start, min, max }`, default `0` in `-5..5`), `rivals` (faction
ids), `art`, `gm_notes`.

**`goals`** — campaign goals to tick off: `id`, `title`,
`description`, `held_by` (a faction), `item`, `gm_notes`.

## What the validator reports

It reports, it never refuses: an import with issues is stored, and the
issues come back with it (`issues`: `severity`, `code`, `path`,
`detail`).

| Code | Severity | Meaning |
| --- | --- | --- |
| `ID_INVALID`, `ID_DUPLICATE` | error | Malformed id; an id used twice anywhere |
| `REF_DANGLING` | error | A reference names nothing |
| `REF_WRONG_KIND` | error | A reference names the wrong kind of thing (an NPC slot pointing at a location) |
| `FORMAT_UNSUPPORTED` | error | `format` is not 1 |
| `AFFINITY_RANGE` | error | Affinity needs `min < max` and `start` between them |
| `THREE_CLUE_RULE` | warning | A critical revelation has clues in fewer than 3 different nodes |
| `REVELATION_NO_CLUE` | warning | No clue leads to a revelation |
| `KNOWLEDGE_NEVER_GIVEN` | warning | A node requires a revelation no other node gives a clue to |
| `KNOWLEDGE_ONLY_OPTIONAL` | warning | …or that only optional scenes give |
| `START_NODE_MISSING`, `NODE_UNREACHABLE` | warning | No opening node; a node no exit path reaches from it |
| `FRONT_CLOCK_LENGTH` | warning | A clock outside 4–6 steps |
| `EXIT_TO_SELF`, `LOOT_EMPTY`, `ENCOUNTER_EMPTY`, `MUSIC_NOT_YOUTUBE`, `FACTION_SELF_RIVAL` | warning | Structural slips |
