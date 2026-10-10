# Map format (version 1)

The format Romain, the map editor and the LLM write. A map is one YAML
file (authoring, `content/maps/`) or the same structure as JSON (stored
in Postgres). Code: `shared/src/maps/` (`Map::from_yaml`,
`Map::from_json`, `Map::validate`). Examples:
`content/maps/corsaires/quai-port-louis.yaml`,
`content/maps/brasier/cure-dent-coursive.yaml`.

**The grid carries the rules.** Walls, doors, props, hidden objects and
lights are data on the grid. A generated or imported image is only a
`backdrop`: it never decides movement or sight.

## Top level

| Key | Required | Meaning |
|-----|----------|---------|
| `version` | yes | `1` |
| `id` | yes | stable id, kebab-case |
| `name` | yes | French name shown to people |
| `scale` | yes | `world` (hexes, ~10 km), `place` (squares, ~5 m), `encounter` (squares, 1.5 m) |
| `cell_meters` | no | overrides the scale's cell size |
| `theme` | yes | tileset / theme pack id (rendering) |
| `ambience` | no | see below; defaults to day, clear |
| `backdrop` | no | `{ prompt, image, cell_px, offset }` — decor only; `prompt` is never sent to players |
| `gm_notes` | no | never sent to players |
| `layers` | no | defaults to one layer `base`, visible to all |
| `grid` | yes | legend and rows |
| `doors`, `props`, `objects`, `lights`, `exits`, `labels`, `starts` | no | lists, below |

Coordinates are `[x, y]`: column then row, `[0, 0]` top-left, north up.
World maps store hexes as rows too: pointy-top hexes, odd rows shifted
half a hex right ("odd-r").

### Backdrop

`image` names the picture drawn behind the grid (`import` for a
campaign map's own image, stored with the map). `cell_px` is the size
of one cell in the image's pixels and `offset` (`[x, y]`, pixels) where
cell `[0, 0]` starts; both must be positive when present. With a
backdrop the renderer draws the image instead of the floor and walls.

## Grid

```yaml
grid:
  legend:
    "#": { terrain: pierre, wall: true }
    ".": { terrain: pavés }
    ",": { terrain: flaque, difficult: true }
    "~": { terrain: eau, water: deep }
    "=": { terrain: pont, elevation: 1 }
  rows:
    - "#####"
    - "#..,#"
```

Each row has the same number of glyphs; every glyph is in the legend
(one character, not a space). A legend entry:

| Key | Default | Meaning |
|-----|---------|---------|
| `terrain` | — | material id, drawn by the tileset (`pavés`, `planches`, `métal`, `herbe`…) |
| `wall` | false | blocks movement and sight; the renderer draws its top one tile up |
| `water` | `none` | `shallow` (difficult terrain) or `deep` (impassable unless the rule system allows swimming) |
| `difficult` | false | entering costs `difficult_factor` (default ×2) |
| `void` | false | nothing there (hole, open space): never walkable, does not block sight |
| `elevation` | 0 | height in levels; one level is drawn one tile up |

Elevation is per glyph: give raised ground its own glyph (`=` for a
ship's deck at 1).

## Layers

```yaml
layers:
  - { id: base, name: Quai, visibility: all }
  - { id: secrets, name: Secrets, visibility: gm }
```

`visibility`: `all` (everyone, shared screen included), `gm` (the GM
only, until revealed — still real for the rules: a hidden trap or a
secret door exists), `players` (what players see in place of the truth:
an illusory wall, a decoy — shown to the GM as an overlay, **ignored by
the rules**). Every placed thing has `layer`, default `base`. Revealing
a layer = setting it to `all`; revealing one object = moving it to a
visible layer.

## Placed things

```yaml
doors:
  - { id: porte-kerjean, at: [3, 3], state: locked, label: Entrepôt de Kerjean }
props:
  - { id: caisses-1, kind: caisses, label: Caisses empilées, at: [7, 5], size: [2, 2], cover: total, blocks_movement: true }
objects:
  - id: trappe
    kind: trappe
    label: Trappe sous l'appontement
    at: [10, 13]
    check: { stat: SAG, dc: 15 }
    notes: Ce qu'elle cache.
    layer: secrets
lights:
  - { id: lanterne, at: [3, 4], bright: 1, dim: 3, color: "#ffb35c", flicker: true }
exits:
  - { id: vers-les-rues, cells: [[0, 4], [0, 5]], to: port-louis-rues, arrive: depart-quai, label: Vers les rues }
labels:
  - { text: Appontement, at: [10, 10], scene: sc-quai }
starts:
  - { id: pj-1, side: party, at: [0, 5] }
  - { id: gueule-rouge, side: foes, at: [13, 6], entity: gueule-rouge, layer: embuscade }
```

- **Doors** sit on a wall cell (square grids only). `state`: `open`
  (passable, transparent), `closed`, `locked` (both block movement and
  sight; locked needs a key or a check to open). A **secret door** is a
  door on a `gm` layer: players receive a plain wall.
- **Props** are decor with rules. `size: [w, h]` from `at` (top-left),
  default `[1, 1]`. `cover`: `none`, `half`, `three_quarters`, `total`
  (total also blocks sight). `blocks_movement`, `difficult`,
  `climbable` (ladder, rigging: lets a creature change height beyond one
  level), `overhead` (drawn above tokens, no rule effect).
- **Objects** are things to find or use (chest, trap, clue, lever).
  `check` (stat id of the rule system, DC) and `notes` are stripped for
  players even when the object is visible.
- **Lights**: radii in cells; `dim` ≥ `bright`.
- **Exits** link cells to another map id; `arrive` names a start or exit
  there.
- **Starts**: token positions; `side` is `party`, `foes` or `neutral`;
  ambushers go on a `gm` layer.

Ids are unique across doors, props, objects, lights, exits and starts.

## Ambience

```yaml
ambience:
  time: night          # dawn | day | dusk | night
  weather: fog         # clear | cloudy | rain | storm | fog | snow | sandstorm
  light: dim           # dark | dim | bright — overrides the time (interiors)
  sight_limit: 10      # cells anyone sees (fog, smoke)
  mood: quai-tension   # music cue and colour grade
  wind: { from: w, strength: 1 }   # n ne e se s sw w nw, 0–5
```

## YAML traps

- A label with a comma inside a `{ … }` flow mapping must be quoted:
  `label: "Sas bâbord, porte intérieure"`. Unknown keys are refused, so
  the mistake fails loudly.
- Quote glyph keys in the legend (`"#"` starts a comment otherwise).

## Rules the engine applies on this data

Parameters come from the campaign's rule system (`MovementRules`:
`diagonal` = `chebyshev` or `alternate` 1-2-1, `difficult_factor`,
`climb_cost`, `max_step`, `swim_factor`).

- **Movement**: 8 neighbours on squares, 6 on hexes. Walls, void,
  closed/locked doors, blocking props and enemies stop a step; allies can
  be crossed, not stopped on. A diagonal never cuts the corner of a wall,
  closed door or blocking prop. Climbing costs `climb_cost` per level up;
  more than `max_step` levels needs a climbable prop. The server checks a
  client's path step by step (`check_path`).
- **Range**: grid distance by the diagonal rule, × cell size for metres.
- **Line of sight**: centre to centre, cell by cell. Blocked by walls,
  closed doors, total-cover props and ground higher than both ends.
  Cover = best of props on the way, creatures (half), grazed wall corners
  (half); obstacles touching the viewer are ignored (one leans over one's
  own cover).
- **Players' projection** (`Map::project(Viewer::Player)`): drops `gm`
  layers and all they hold, `gm_notes`, object checks and notes, and the
  backdrop prompt. Fog of war is cut on top of it.

## World maps and their travel guide

A `world` map is travelled by the party as **one token**
(`maps/travel-hex-world`, `shared/src/travel/`). The map stays the grid
above; how it is crossed lives next to it, in a travel guide
(`content/travel/<world>/<map>.yaml`, `travel::Guide`), so the same hexes
could be sailed with one guide and walked with another:

- `portions` — the portions of a travelling day (`[Matin, Après-midi,
  Soir]` by default); night follows the last one;
- `steps_per_portion` (2) — steps covered per portion; entering a hex
  costs its terrain's steps, the leftover carries over;
- `supplies` — `{ name, per_person_per_day }`, eaten at each dawn;
- `watches` — the night's watches, in order;
- `terrains` — by legend `terrain` id: `name` (for people), `cost`
  (steps), `impassable`, and `events` (`id`, `title`, `text` read to the
  table, `gm_notes` never sent to players);
- `events` — a table for any terrain.

Without a guide a world map travels by its cell flags: 1 step, 2 through
difficult terrain or shallow water, nothing through walls, voids and
deep water. Places are the map's `labels` (`scene` = the story node the
place opens); an `exit` on a label's hex is the map the place opens on
arrival. The party starts on the map's first `party` start.

## Editing by operations

The editor and Claude never write a map whole: they send batches of
operations (paint, place, move, remove, erase, door, set) on the
campaign revision they read — `shared/src/maps/edit.rs`, protocol in
`docs/claude-mcp.md` under *Map operations*. The result goes through
the same `Map::validate` as a file. Placed things get ids by prefix
(`porte-n`, `decor-n`, `objet-n`, `lumiere-n`, `depart-n`); a new kind
of cell takes the first free glyph of `.#,~_=:;+*%&@!?a…z A…Z 0…9`, and a
glyph no cell uses leaves the legend.

## The atlas: how maps nest

The GM's Atlas (`maps/browse-atlas`, `shared/src/maps/atlas.rs`) shows
every map of the world (`content/maps/<world>/`) and of the campaign
(`campaign_maps`) as one tree — region in hexes › place › encounter —
read from the maps' `exits`, never stored:

- An exit from A to B makes A the **parent** of B only when B's `scale`
  is strictly finer (`world` › `place` › `encounter`; a world map may
  hold an encounter directly). The exit's `cells` are where B is
  anchored on A: one hex on a region, a zone on a grid.
- An exit between two maps of the same scale, or back up, is a **link**
  (« à côté »), never an edge. Every edge goes down a scale, so the tree
  has no cycle whatever the exits say.
- An exit to a map neither the world nor the campaign has is ignored.
- A map several exits lead down to keeps one parent: the nearest
  coarser scale first (a place before the region for an encounter),
  then a campaign map before a world map, then the smallest id. A map
  no exit leads down to is a root; a campaign map shadows a world map
  of the same id.

`GET /api/campaigns/{id}/atlas` (GM only, not open to a personal token)
answers `{revision, roots, maps: [{map, origin: world|campaign, parent:
{map, exit}|null, children, links: [{to, exit}], backdrop, validated}]}`,
maps whole, roots first then depth first. A player never receives it:
the table shows the one map on the board, through the projection.

The story's places will anchor maps too (`campaign/pin-scenes-on-maps`);
until then a GM's own map hangs under another only if that map has an
exit to it (a copy of a world map keeps the exits it had).

## Universal VTT import

`maps::from_uvtt` reads a Universal VTT export (`.dd2vtt`, `.uvtt`,
`.df2vtt`): `resolution.map_size` gives the grid,
`pixels_per_grid` the backdrop's `cell_px`. Walls (`line_of_sight`,
`objects_line_of_sight`) are segments there and cells here: each
segment is sampled every 0.1 cell and a sample on a cell edge walls the
cell left of or above it. Portals become closed doors on cells made
walls. Lights become lights: `range` → `dim` (rounded up), `bright` half
of it, `ffRRGGBB` → `#rrggbb`. The base64 `image` is kept as the
backdrop.

## V1 maps

`load_v1_story_maps` reads the `maps` array of a V1 story (JSON):
`campaign` → world, `region` → world with 500 m hexes, `local` →
encounter; blocked water → deep water, blocked labelled cells on squares
(throne, campfire) → blocking props, other blocked cells → walls;
`childMapId` → exits; tokens → starts. The demo campaign's four maps are
in `content/maps/v1-demo/`.
