# Claude MCP access (`platform/connect-claude-mcp`)

The GM prepares a campaign by talking to their own Claude (Claude
Desktop or Claude Code on their computer). Claude launches a small
local MCP server (`mcp/`, stdio) that calls the Promptus API with a
**GM personal access token**. Claude on the web is out of scope: it
would need a server reachable from the Internet with OAuth.

```
Claude Desktop / Claude Code ──stdio──> mcp/ (bun) ──HTTPS + Bearer──> Promptus /api
```

## Tokens

- Minted, named, listed and revoked from the GM home (« Accès pour
  Claude »), through `GET/POST /api/gm-tokens` and
  `DELETE /api/gm-tokens/{id}`. These routes take the GM's session
  cookie only (`SessionOnly`): a token never mints or revokes a token.
- The secret is `promptus_` + 32 random bytes (base64url), shown once
  in the `POST` answer. The table `gm_api_tokens` stores its SHA-256;
  the lookup is a unique-index hit on the hash, so nothing compares the
  secret byte by byte. Each accepted call stamps `last_used_at`.
- Revoking sets `revoked_at`: the next call answers 401
  `INVALID_TOKEN`. A GM holds at most 20 live tokens. No expiry: the
  GM revokes.

## What a token reaches

`require_gm` reads `Authorization: Bearer` before the cookie. With a
bearer header, the matched route must be in
`auth::api_tokens::TOKEN_ROUTES`, checked before any database lookup;
anything else is 403 `TOKEN_NOT_ALLOWED`.

| Route | MCP tool |
| --- | --- |
| `GET /api/campaigns` | `list_campaigns` |
| `POST /api/campaigns/import` | `create_campaign_from_yaml` (a campaign written with Claude lands as a new one, in preparation) |
| `GET /api/campaigns/{id}` | `get_campaign`, `get_scene`, `get_readiness` (validator issues) |
| `GET /api/campaigns/{id}/readiness` | `get_readiness` (act gauges) |
| `POST /api/campaigns/{id}/story/edits` | `apply_story_edits` (proposed, below) |
| `GET /api/campaigns/{id}/export` | `export_campaign_yaml` |
| `PUT /api/campaigns/{id}/import` | `import_campaign_yaml` (only with « Claude a la main », below) |
| `GET /api/campaigns/{id}/maps` | — (the campaign's own maps; `get_atlas` covers them all) |
| `GET /api/campaigns/{id}/maps/{map}` | `edit_map` (reads the `revision` first) |
| `POST /api/campaigns/{id}/maps/{map}/ops` | `edit_map` (a batch of map operations, below — proposed too) |
| `GET /api/campaigns/{id}/atlas` | `get_atlas` (every map nested, what stands on each, the places, the scenes without a pin) |
| `GET /api/campaigns/{id}/maps/{map}/sketch` | `get_map_text` (the map as text, below; a world map too) |
| `POST /api/campaigns/{id}/maps/{map}/ask` | `ask_map` (path and cost, line of sight, what players see) |
| `POST /api/campaigns/{id}/maps/{map}/place` | `place_on_map` (pins, NPCs, clues, fights placed by name — proposed too) |
| `GET /api/campaigns/{id}/proposals` | `list_proposals` (Claude's batches and the GM's word on each item) |

Deliberately excluded: sign-in, accounts, the token routes, creating
an empty campaign, archiving / setting a campaign, `story/validate`
(declaring the campaign playable is the GM's decision; the validator's
report already comes with every read and edit), the table and players,
the live session and its socket, player and screen routes, and every
route that spends AI budget (generation, workshop, generated maps,
images, co-GM). `tests/gm_routes_test.rs` sends a live token to every
other GM route and expects 403; `tests/api_tokens_test.rs` covers the
in-scope routes, revocation and ownership (another GM's campaign is
404).

A scene's spot on a map is a story edit too (`campaign/pin-scenes-on-maps`):
`{ "op": "set", "target": "<location>", "field": "anchor", "value": { "map":
"<map id>", "at": [x, y] } }` pins the place (and every scene up to it);
`"value": null` takes the pin off. Whether the map and the cell exist is
reported by the atlas, not refused.

NPCs, clues and fights are placed the same way (`campaign/place-cast-and-clues`):
an NPC's `spots` is the whole list (`{ "op": "set", "target": "<npc>",
"field": "spots", "value": [{ "map": "<map id>", "at": [x, y], "scene":
"<node>" }] }`, `scene` left out for every scene on that map — read the
list first, the set replaces it); a clue's `anchor` and a scene's
`encounter.anchor` take `{ "map", "at" }` or `null`. A spot inside a wall
is reported (`SPOT_ON_WALL`), never refused. Nothing placed reaches the
table until the GM shows the map and reveals what is there.

## Claude proposes, the GM keeps (`platform/propose-edits-by-claude`)

A token write is checked exactly like the GM's (same edit format and
map operations, same lock, same all-or-nothing, same errors) and then
**proposed**, not applied: `202` with

```json
{ "data": { "status": "proposed", "why": "proposals", "proposal": { … }, "issues": [ … ] } }
```

The batch is a row of `story_proposals` (the co-GM workshop's table,
one model of batch: `author = 'token'`, migration 049): its title
(`title` in the body, stored in `prompt`), its focus on a map (`focus:
{ map, at? }` in the body; a map batch is focused on its map and the
first cell it touches), its story edits and its map batches (each with
the revision it was checked at). Items have keys: `e<i>` for the i-th
story edit, `m<i>` for the i-th map batch. `proposal: "<id>"` in the
body adds to a batch still pending (checked on the story as the batch
already leaves it, so a clue added in it can be placed by the next
call); a settled one answers 409 `PROPOSAL_ALREADY_DECIDED`. `why` says
why it was proposed: `proposals` (the default), `evening` (an evening is
open), `revealed` (it touches what players know), `batch` (added to a
batch).

The GM, in Promptus only (`SessionOnly`, not a token route), decides
with `POST /api/campaigns/{id}/proposals/{proposal}/decide`:
`{ "keep": ["e0"], "discard": ["e1"], "word": "trop évident" }`, or
`{ "all": "keep" }` / `{ "all": "discard", "word": "…" }`. Keeping
replays under the campaign lock on the campaign as it is now: kept
story edits as one journal line, each map batch as one through the
editor's own rules (`campaign_maps::prepare`, so a cell the GM painted
since refuses it), all by author `copilot` (a proposal the GM accepted —
no « Claude écrit » activity). Keeping named items is strict: one that
no longer applies answers 409 `PROPOSAL_ITEM_STALE` and nothing is
written. Keeping everything left refuses what no longer applies one by
one (`status: "refused"`, `code`: why) and keeps the rest. A batch is
`pending` while an item waits, then `accepted` (at least one kept) or
`rejected`.

Claude reads the outcome with `GET …/proposals` (the latest 30,
newest first): each item's `status` (`pending`, `kept`, `discarded`,
`refused`), the GM's `word`, a refusal `code`, `stale` when a pending
item no longer applies; the batch's `feedback` is the word on « tout
jeter ». The co-GM workshop's list (`GET …/workshop`) shows only the
co-GM's proposals.

The GM's screens hear of a batch on the GM-only `proposals` live
topic (created, decided, the mode switched); players' and screens'
sockets never do, not even in the versions sent on connection
(`live::visible_to`). The atlas (`GET …/atlas`) carries `claude`, the
pending batches oldest first, and on each map `proposed`: what each
batch would add there, applied virtually on the story and the maps —
pins, NPCs, clues, fights, thread segments not on the map yet, and the
cells its map operations touch — drawn dotted and marked ▶. Nothing of
it is stored in the story or the maps until kept.

**« Claude a la main »** — `PUT /api/campaigns/{id}/claude`
`{ "hand": true }`, the GM's session only (a token never gives itself
the hand), refused while an evening is open (409 `EVENING_OPEN`);
taking it back always works. With it, a token write lands directly
(`200`, `status: "applied"`), journaled as `token` and undone like any
line of the journal (one MCP call, one revision) — except while an
evening is open and on what players already know (a scene entered, a
clue found, an NPC or adversary met, a faction or goal known, a place
where an entered scene stands or one containing it, the map on the table's board): then it
is proposed. A whole file over the campaign (`PUT …/import`) cannot be
proposed item by item: with a token it is refused (409
`CLAUDE_PROPOSES`) unless Claude has the hand. This closes the open
question of `platform/connect-claude-mcp`: during an evening a token
only ever proposes.

Every story write — a token's in the hand included — takes the campaign's next
preparation revision and leaves a line in its journal
(`campaigns::revisions`, `platform/stream-prep-edits`): author (`token`
here), the entities touched, and each as it was. The GM's open prep
screen hears the revision on the GM-only `prep` live topic, plus an
`activity` message for a token write, reads
`GET /api/campaigns/{id}/revisions?since=N`, shows « Claude écrit · sur
… », lights up what moved, and may take the write back
(`POST …/revisions/{n}/undo`) while no later revision touched the same
entities. Players' and screens' sockets never hear either. The journal
and undo routes are not token routes: a token never takes back a write.

## The MCP server (`mcp/`)

TypeScript on Bun with `@modelcontextprotocol/sdk`: the official SDK,
no build step (Bun runs the `.ts` entry directly), and the same runtime
as the rest of the repo's tooling. A Rust binary would have needed a
second, younger MCP crate and a compile on the Mac for no gain.

- Configuration: `PROMPTUS_URL` (the origin the app is opened at, no
  `/api`) and `PROMPTUS_TOKEN`. When either is missing the server still
  starts and every tool answers what to set.
- stdout is the protocol: logs go to stderr only.
- Errors come back as tool errors written in French for the GM:
  revoked token, out-of-scope route, unknown campaign, unreachable
  server, refused edit (with the server's `edit N: …` detail), a whole
  file without the hand (`CLAUDE_PROPOSES`), a settled or unknown batch.
- `apply_story_edits` takes `title` and `proposal_id` and says whether
  the edits were proposed (and why) or applied; `list_proposals` gives
  Claude's batches back with what the GM kept, discarded (and the word)
  or saw refused.
- The map tools (`mcp/src/maps.ts`, `platform/edit-maps-by-claude`):
  `get_atlas`, `get_map_text`, `ask_map`, `place_on_map`, `edit_map`
  (below, « Maps in words »).
- Tests: `cd mcp && bun test` — a real MCP client over the SDK's
  in-memory transport, the API stubbed by a mocked `fetch`. CI runs them
  with `tsc --noEmit` (job `mcp`).

## Declaring it

Once per machine: `cd <checkout>/mcp && bun install`.

Claude Desktop — `~/Library/Application Support/Claude/claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "promptus": {
      "command": "/Users/<you>/.bun/bin/bun",
      "args": ["run", "/path/to/DnD-promptus/mcp/src/index.ts"],
      "env": {
        "PROMPTUS_URL": "https://promptus.example",
        "PROMPTUS_TOKEN": "<token shown once>"
      }
    }
  }
}
```

Claude Desktop starts servers with a minimal `PATH`: give bun's full
path (`which bun`). Claude Code:

```bash
claude mcp add promptus -s user -e PROMPTUS_URL=https://promptus.example \
  -e PROMPTUS_TOKEN=<token> -- bun run /path/to/DnD-promptus/mcp/src/index.ts
```

The GM home shows both snippets with the app's origin, and with the
token filled in right after it is minted.

## Map operations

A map is never written whole (`maps/edit-maps-by-ops`). A writer reads
it (`GET …/maps/{map}`, whose `revision` is the campaign's preparation
revision) and sends `POST …/maps/{map}/ops` with
`{ "revision": N, "ops": [...] }`, at most 200 operations applied in
order, all or nothing (`shared/src/maps/edit.rs`, `MapOp`):

| `op` | Fields | Effect |
| --- | --- | --- |
| `paint` | `cells`, `kind` (a legend entry) | cells off the grid ignored; a door whose wall is painted over leaves |
| `place` | `at`, `thing` (`prop` `{prop,label}`, `object` `{label}`, `light`, `start` `{side, entity?}`), `layer` | a new id: `decor-n`, `objet-n`, `lumiere-n`, `depart-n` |
| `move` | `id`, `to` | a door, prop, object, light or start |
| `remove` | `id` | any id'd thing, exits included |
| `erase` | `at` | everything placed on the cell |
| `door` | `at`, `layer` | new (closed, cell walled) → open → locked → gone |
| `set` | `name?`, `notes?`, `ambience?` | the map's own fields; empty `notes` clears |

A batch on a stale revision applies to the map as it is now unless a
later batch on the same map touched the same cell or thing: then 409
`MAP_STALE` with `error.fresh` (the map and its revision), and nothing
is written. 400 `MAP_OP_INVALID` / `MAP_INVALID` say in French what is
wrong. Each batch is a journal line (`target: "map:<id>"`, changes
`{op, kind: cells|door|prop|object|light|exit|start|map, id, name,
cells}`) and the GM may take it back from the journal; a token write's
`activity` reaches the GM's screens like a story write. A token batch
is proposed (above): the body may carry `title` and `proposal`, the
answer is `202` with the batch; it applies at once only when Claude has
the hand and the map is not on the table.

## Maps in words (`platform/edit-maps-by-claude`)

Claude does not see pixels. It reads and places by name, and the server
does the geometry (`shared/src/story/sketch.rs`, `placing.rs`;
`back/src/campaign_maps/survey.rs`). A map is looked up among the
campaign's own maps, then its world's (`content::map`): a world map is
read and receives pins and NPCs, but is never redrawn.

**`GET …/maps/{map}/sketch?x=&y=&w=&h=`** — the map as text, the whole
map or a zone (80 × 50 cells at most; `cut` says the zone is smaller
than the map). `rows` are two ruler lines (tens and units of `x`), then
one line per row, its `y` right-aligned on three characters and a
space, then one character per cell — the grid's own glyph, or a symbol
over what is placed there. Hex maps take two characters per cell and
shift odd rows by one (odd-r offset). `terrain` is the grid's legend for
the glyphs drawn (`terrain`, `wall`, `void`, `water`, `difficult`,
`elevation`). `legend` lists every mark in the zone: `{symbol, kind:
pin|npc|clue|fight|exit|door|prop|object|start|label, id, name, cells,
layer?: all|gm|players, note?}` — the preparation's marks first, in
capitals and digits, the map's things in lowercase and punctuation,
never a glyph of the grid's legend; a party's starts without an entity
share one mark. The words (« Épingle », « mur ») are the MCP server's.

**`POST …/maps/{map}/ask`** — `{question: "path", from, to}`,
`{question: "sight", from, to}` or `{question: "view", from}`.
References are strings: a cell `"x,y"`, or by exact id, then by name,
then by a part of a name (accents, case and a leading article aside) a
prop, object, door, exit (label or destination), label, start (id or
entity) of the map, or a pin (its place or one of its scenes), a placed
NPC or a placed clue. Two different things at one level answer 400
`MAP_REF_AMBIGUOUS` with the candidates in `message`; nothing answers
`MAP_REF_UNKNOWN` with what the map knows; a cell off the grid
`MAP_REF_OFF_GRID`. Answers: `path` (the walk from the cell nearest
`from` to the cell nearest `to`, both ends included, `cost` in cells of
movement under the campaign's movement rules, `meters`; no `cost` when
there is no walk); `sight` (`viewer`, `target`, `visible`, `cover`,
`cells`, `meters`); `view` (`viewer`, `seen` cells, `light`, and a
`sketch` of the players' projection fogged beyond what they see).

**`POST …/maps/{map}/place`** — `{placements: [{what, id, scene?, near?
| along?: {from, to}}], title?, proposal?}`, 50 at most, all or nothing.
`what`: `scene` (its place's `anchor`), `place`, `npc` (one more spot;
`scene` for one scene only), `clue`, `fight` (the scene's
`encounter.anchor`; not a ship battle, `PLACE_NO_FIGHT`). The server
walks out from the reference and takes the first cell that is
standable and out of water (shallow or deep), free (no mark of the
preparation, start, object, prop, exit or door, and no mark still
proposed in another waiting batch) and reachable from the party's
starts and the exits (any cell when the map has neither); it never
walks through a wall, so « near the counter » stays on the counter's
side. Placements `along` the same path are spread evenly on the walk
between its two ends, in order. Pinning a scene anchors its place (the
place's other scenes follow, `with`); a scene with no place, or whose
place is pinned on another map, gets a place of its own (`created`,
`lieu_<scene>`, inside the old one) so the other pin stays; pinning a
`place` pinned elsewhere is refused (`PLACE_ANCHORED_ELSEWHERE`). The
result is the story edits the GM would make (`set anchor`, `set spots`
with the whole list, `set encounter.anchor`, `add location`): from the
GM's session they apply; with the token they go through
`prep::claude::story_edits_with`, resolved under the campaign lock on
the story the named batch would leave, so they are proposed like any
of Claude's edits. The answer is that of `story/edits` plus `placed:
[{what, id, name, at, near, steps, terrain, created?, with?}]`. Errors:
`PLACE_MALFORMED`, `PLACE_UNKNOWN_ENTITY`, `PLACE_NO_ROOM` (no free,
dry, reachable cell), `PLACE_NO_PATH`, the reference codes,
`TOO_MANY_PLACEMENTS`, with an English `message` naming the placement's
index.

`edit_map` reads the map's `revision`, sends the operations, and on 409
`MAP_STALE` replays once on `error.fresh.revision`.

⚠️ World maps flag no water: their sea is a terrain (`mer`), so
« never in the water » holds on place and encounter maps, whose water is
flagged, not on a region's hexes.
