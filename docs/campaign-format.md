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
| `party` | no | Player character slots: `id`, `name`, `player`, `concept`, `class` (a class id of the rule system, when the prep assumes one) |
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
| `map` | The grid map the scene is played on: a map file's `id` (`content/maps/`), not a campaign id |
| `read_aloud` | Shown or read to the players on entering |
| `ambience` | `mood`, `sounds`, `music`: list of `{ mood, title, url, search }` — `mood` is `calm`, `exploration`, `tension`, `mystery`, `combat` or `epic`; `url` a YouTube link; a track still to choose has no `url`, only a `search` hint, and never reaches players |
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
`faction`, `location`, `permanent` (`true`: always with the party,
like the Brasier's ship AI — the co-GM may speak for them in any scene),
`gm_notes`.

**`stats`** (NPCs and adversaries) — `abilities` (map of the rule
system's ability ids to scores), `armor_class`, `hit_points`, `attacks`
(`{ name, damage, notes }`, `damage` a formula like `1d6+2`). The rules
engine checks these against the campaign's rule system.

**Rules live in one copy.** When the rule system already has the stat
block, `stats: { from_rules: <adversary id> }` names it and the numbers
stay in the rule system; fields written next to it override it. An item
of the campaign likewise names its rule-system item with `from_rules`
(the shop's prices and descriptions stay in the campaign, the rule
effects in the rule system).

**`adversaries`** — stat-block creatures without an NPC sheet: `id`,
`name`, `description`, `art`, `stats` (required), `gm_notes`.

**`locations`** — `id`, `name`, `description` (what players see),
`parent` (the containing location), `art`, `gm_notes`.

**`items`** — `id`, `name`, `description`, `effect`, `value`, `rarity`
(`common`, `uncommon`, `rare`, `epic`, `legendary`, `divine`), `art`
(the 12 × 12 sprite), `gm_notes`.

**`factions`** — `id`, `name`, `description`, `diplomacy`, `affinity`
(`{ start, min, max }`, default `0` in `-5..5`), `rivals` (faction
ids), `art`, `gm_notes`. In play the GM moves a faction's gauge from
the evening screen: winning `n` points with a faction costs `n` with
each of its rivals (bounded), losing favour moves no one else. Players
see a faction once met: its name and gauge only.

**`goals`** — campaign goals to tick off: `id`, `title`,
`description`, `held_by` (a faction), `item`, `gm_notes`. Players see
each goal's title and whether it is reached.

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
| `EXIT_TO_SELF`, `LOOT_EMPTY`, `ENCOUNTER_EMPTY`, `MUSIC_NOT_YOUTUBE`, `MUSIC_NO_URL`, `FACTION_SELF_RIVAL` | warning | Structural slips |
| `PLAYER_WITHOUT_HOOK` | warning | A party member has no hook in an act that has scenes |

### Against the rule system and the maps

`story::validate_with(campaign, &Library { rules, maps })` runs the
checks above, then those that need what the campaign points at. Each
part is optional; what is not supplied is not checked.

| Code | Severity | Meaning |
| --- | --- | --- |
| `RULES_MISMATCH` | error | The rule system supplied is not the one `rules` names (nothing else is checked against it) |
| `RULES_UNKNOWN_CLASS`, `RULES_UNKNOWN_ADVERSARY`, `RULES_UNKNOWN_ITEM`, `RULES_UNKNOWN_ABILITY` | error | A `class`, `from_rules` or check `stat` names nothing in the rule system |
| `DIFFICULTY_OFF_SCALE` | warning | A planned or clue check difficulty that is none of the rule system's named difficulties |
| `MAP_UNKNOWN` | error | A node's `map` is no known map |
| `MAP_ENTITY_UNKNOWN` | warning | A map a node uses places a token whose `entity` is no NPC or adversary of the campaign |

Two more checks of the campaign alone came with the witness worlds:
`PLAYER_WITHOUT_HOOK` (warning: a party member has no hook in any scene
of an act that has scenes) and `MUSIC_NO_URL` (warning: a track still
to choose).

## Is an act ready?

`story::readiness(campaign, &Library)` gives one gauge per act
(`docs/lecons-des-parties.md` §3, chain 2). It reports, never blocks.

| Check | Passes when | Gap codes |
| --- | --- | --- |
| `scene_fields` | Each scene has `summary`, `location`, `ambience.mood`, `read_aloud`, `flow`, and `transition` when it has exits | `MISSING_FIELDS` (with `fields`) |
| `knowledge_paths` | Each revelation a scene of the act `requires`, and each critical one the act gives clues to, is given in at least 3 scenes other than the one requiring it, not all optional | `FEW_PATHS` (with `paths`), `ONLY_OPTIONAL` |
| `player_hooks` | Each party member has a hook in a scene of the act | `NO_HOOK` |
| `encounters` | Each planned fight's opponents have numbers (`from_rules`, or hit points and armour class) and the fight has a tactic | `NO_STATS`, `NO_TACTICS` |
| `fights` | Each planned fight can be staged (`encounter_scenario`) and simulated on its map | `NOT_STAGED`, `SIMULATION_FAILED`, `NO_RULES` |

An act with no scene is not ready.

## Editing by id

The review screen (`/campagnes/:id/preparer`) and the co-GM's workshop
change a stored campaign with **edits by id** (`story::edit`,
`POST /api/campaigns/{id}/story/edits`), applied whole or not at all:

```json
{ "op": "set", "target": "sc_quai", "field": "summary", "value": "…" }
{ "op": "set", "target": "bible", "field": "truths", "value": ["…"] }
{ "op": "set", "target": "pnj_morel", "field": "stats.hit_points", "value": 12 }
{ "op": "add", "kind": "clue", "value": { "id": "cl_…", "revelation": "…", "node": "…", "text": "…" } }
{ "op": "remove", "target": "cl_…" }
```

`target` is an id, `bible` or `campaign` (title, universe); a dotted
`field` reaches inside an object, and `null` clears it. `id` and
`format` are never set. The result is read back through the model, so
an unknown field or a wrong type is refused like an import
(`EDIT_INVALID`, with the edit's index). What the result breaks is the
validator's to report.

The co-GM's proposals are the same edits. Before the GM sees one, each
edit that does not apply or adds an error (an id it invented, a
reference to nothing) is dropped and counted. Accepting re-applies the
edits to the campaign as it is then; one the campaign has outgrown is
refused (`PROPOSAL_STALE`).

A campaign is **validated** (`validated_at`) when the GM declares it
playable, which needs no error left; only a validated campaign opens a
session. An import is validated as it arrives (the GM wrote it); a
campaign created from a pitch, or generated, waits for the GM.

## Generated from a pitch

`/campagnes/:id/generer` (`ai/generate-campaign`,
`back/src/prep/generation.rs`) turns the GM's idea into a whole
campaign, in a background job the screen follows live:

| Step | The model writes | Template |
|------|------------------|----------|
| `cast` | bible, acts, fronts, NPCs, adversaries (`from_rules` from the rule system), places, items, factions | `generation-cast` |
| `scenes` | nodes, revelations, clues, opening node | `generation-scenes` |
| `check` | repair edits for what the validator finds (errors, three-clue rule, unreachable scenes, knowledge never given) — at most 2 rounds, a round kept only when it makes nothing worse | `generation-repair` |

Each answer is read straight into the model types; one out of format
goes back once with serde's complaint. Ids the model invented —
references to nothing, ids declared twice, rule-system ids the rule
system lacks — are removed (`story::prune`): an optional reference is
emptied, an element that cannot stand without it (a clue in no scene,
an exit to nowhere) is dropped, and the job lists what went. Repair
edits go through `edit::sanitize` like the co-GM's.

Every call is counted (`purpose = generation`): the job is refused
before its first call when its estimate (each step's prompt and whole
answer, one repair) passes what is left of the budget, and stops when
a later call would. The result is a draft (`generation_jobs.draft`):
**applying** it replaces the campaign's story, keeping its id, title,
universe, rule system and party, and the campaign is still to review
and validate. A validated campaign is not generated over.

## The witness worlds and `bun run worlds`

The two worlds live in `content/campaigns/<world>/campagne.yaml`, next
to `content/rules/<world>/` and `content/maps/<world>/`. `bun run
worlds` loads every rule system, map and campaign under `content/`,
runs the rule lint and balance report, the map checks and
`validate_with`, and prints a French report per world. It exits
non-zero only when a file does not load; checks never block.

The Corsaires' act 1 as played is kept as a test case,
`content/fixtures/corsaires-acte-1-joue.yaml`: the checks must keep
finding its defects (`shared/tests/witness_worlds.rs`).

## Importing a V1 campaign

`story::v1::import_v1(entities_yaml, story_json, rules)` converts a V1
campaign — its entity export (YAML, single, list, `{ entities }` or a
multi-document stream) and its story JSON — into one campaign, and
lists what it dropped (spells, events, conditions, triggers, effects).
The V1 demo is `content/fixtures/v1-demo/`; its maps are converted by
`maps::load_v1_story_maps` (`content/maps/v1-demo/`).
