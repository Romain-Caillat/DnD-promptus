# Design canvas — sources and how to work on them

The design lives in a private Claude design canvas:
https://claude.ai/artifact/AP3z1S5hbhqEiPzi5agTdy (owner: Romain).
`canvas/` is a snapshot of its sources (version 66, 3 October 2026).
The published canvas is the reference: if they differ, read the canvas
back before editing, Romain moves and resizes boards there.

Decisions are in `MEMORY.md` §2, remaining work in `TICKETS.md`
(epic `design`), journeys in `docs/user-journeys.md`.

## Rendering check

Every board was checked rendered in Chrome on 3 October 2026 and
fixed; what remains is listed in `design/verify-canvas-rendering`.
Chrome must stay in the foreground while checking, and has reduced
motion on (`MEMORY.md` §4). To see one board large, set the index's
`launch` to `{"view":"focused","file":"<board>"}`, publish, reload;
put `{"view":"canvas"}` back when done.

## Board format

- One `*.dc.html` per board, `canvas.json` for layout (`boards`: x, y,
  w, h, title, `is_interactive`; `order`; `notes`).
- Markup inside `<x-dc>`, styles in `<helmet>`, logic in
  `<script type="text/x-dc">` as `class Component extends DCLogic` with
  `renderVals()` and `this.setState`.
- Template holes `{{name}}`, `<sc-if value>`, `<sc-for list as>`,
  handlers `onClick="{{fn}}"`. Holes hold text only, never HTML.
- Components are boards imported with
  `<dc-import name="Objet" … hint-size="48px,48px">`.

## Traps

- **A board's CSS reaches the components it imports.** Every board must
  prefix its classes and keyframes: `scope.py` does it
  (`scope(html, prefix, keep=…)`; `keep` lists runtime state modifiers,
  always combined with a prefixed class). Done for the evening boards
  only; see `design/scope-legacy-boards`.
- **Python f-strings eat braces:** a hole is written `{{{{name}}}}`.
- `support.js` (the canvas runtime) is not in the snapshot.

## Boards

| File | Title | Notes |
| --- | --- | --- |
| Main | Fondations | colours, type, materials |
| Histoire | Construction d'histoire (LLM) | GM prep, interactive |
| MJ | Écran MJ — en direct | GM live, first draft, superseded by `Mener-MJ` |
| TV | Écran TV | wrapper on `Ecran-TV`, combat moment |
| Joueur-Scene / -Combat / -Carte / -Personnage / -Journal | player phone screens | wrappers on `Jouer-Marc` frozen on a moment and tab |
| Pistes-UI, Raretes, Objets, Etats, Notifs | design system boards | |
| Cartes, Exterieurs | maps, three scales, outdoor | outdoor PNGs embedded as data URIs |
| Parcours | user journeys | |
| Jouer-Marc | **Jouer · la soirée de Marc** | the playable evening: single source of truth for the player phone, TV shown beside it |
| Soiree-Marc | Joueur · la soirée de Marc | storyboard, 12 `Jouer-Marc` instances (`seul=true`) |
| Ecran-TV | Composant — écran TV (soirée) | TV driven by moment and player actions |
| Soiree-TV | TV · la soirée côté TV | storyboard, 12 `Ecran-TV` frames |
| Mener-MJ | **Mener · la soirée de Marc côté MJ** | the playable evening on the GM laptop (1440 × 900), TV beside it; `seul=true` shows the laptop alone, frozen on `moment` |

Components: GameCard, RadialGrid, Carte, Sprite, De, Gemme, Coeurs,
Cases, Horloge, Bouton, Objet, Etat, Perso, Plan.

## Generators

Run from the repository root; they rewrite files in `canvas/` and keep
any board layout already set in `canvas.json`.

- `player-evening-play.py` → `Jouer-Marc` (uses `player-evening.css`).
- `tv-evening.py` → `Ecran-TV`.
- `gm-evening-play.py` → `Mener-MJ`.
- `outdoor/*.py` → outdoor map PNGs; `items-prototype.py`,
  `walls-prototype.py`, `sprite-prototype.py` (+ `sprites.json`) →
  pixel-art previews.

Other boards were edited by hand. Regenerating `Jouer-Marc`,
`Ecran-TV` and `Mener-MJ` from these scripts reproduces the snapshot byte for byte.

## Publishing

Publish with the Artifact tool to the canvas URL above, root = the
folder holding `canvas/` contents. A publish is refused when a file
changed remotely since it was read: read it, merge, publish again.
Do not overwrite `canvas.json` if Romain changed the layout.
