# Design canvas — sources and how to work on them

The design lives in a private Claude design canvas:
https://claude.ai/artifact/AP3z1S5hbhqEiPzi5agTdy (owner: Romain).
`canvas/` is a snapshot of its sources (version 81, 4 October 2026).
The published canvas is the reference: if they differ, read the canvas
back before editing, Romain moves and resizes boards there.

Decisions are in `MEMORY.md` §2, the closed design epic in
`archive/tickets/design.md`, journeys in `docs/user-journeys.md`.

In the app, the values of the « Fondations » board live in
`front/src/styles/tokens.css` (Tailwind v4 `@theme` tokens and material
utilities); the route `/reference` shows them. Change a colour, font or
duration there, not in components. The interface chrome follows track A
« Menu pixel » of `Pistes-UI` (ui/adopt-pixel-menu, `MEMORY.md`, visual
direction), not the card buttons of track B that the board marks as
retained: the board predates that choice.

## Rendering check

Every board up to 3 October 2026 was checked rendered in Chrome and
fixed. The boards of 4 October were only checked by their logic tests:
the design epic is closed (`archive/tickets/design.md`), and the rest is
checked on the real platform.
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
  always combined with a prefixed class). Done for every board since the
  evening; the older boards were left as they are (`archive/tickets/design.md`).
- **Python f-strings eat braces:** a hole is written `{{{{name}}}}`.
- `support.js` (the canvas runtime) is not in the snapshot.

## Boards

| File | Title | Notes |
| --- | --- | --- |
| Main | Fondations | colours, type, materials |
| Preparer-MJ | **Préparer · la campagne de Romain** | the playable GM prep journey (1440 × 900), ten moments from the campaign list to a playable campaign; the story workshop is moment 6 |
| Preparation-MJ | MJ · préparer une campagne | storyboard, 10 `Preparer-MJ` frames (`seul=true`) |
| Inviter-MJ | **Inviter · la table de Romain** | GM laptop and Marc's phone side by side, seven moments from the invite link to the first session date; `seul=true` is 1880 × 900 |
| Invitation-MJ | MJ · inviter la table | storyboard, 7 `Inviter-MJ` frames |
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
| Soiree-MJ | MJ · la soirée côté MJ | storyboard, 12 `Mener-MJ` frames (`seul=true`) |
| Creer-Joueur / Creation-Joueur | **Créer · le personnage de Marc** | phone, 8 moments: layered dwarf (`avatar.py`), class, flagged limit, story with the co-GM |
| Entre-Joueur / Entre-Sessions | **Entre deux · Marc entre les sessions** | phone, 6 moments: level 4, recap, chronicle, sheet, dates |
| Lancer-MJ / Lancement-MJ | **Lancer · la TV et la session 4** | laptop and TV, 5 moments: pairing code, what the TV may show, « Précédemment… » |
| Voyager-MJ / Voyage-MJ | **Voyager · de Valombre à Morneval** | laptop and phone, TV in the side panel, 7 moments on the hex map |
| Regles-MJ / Systeme-Regles | **Règles · le système de Valombre** | laptop, 7 moments: SRD copy, house rule formalised, simulation |
| Carte-MJ / Editeur-Carte | **Cartes · l'éditeur de la salle de l'autel** | laptop, 7 moments over the `Plan` component |
| Joueur-Ordi / Ordi-Sef | **Jouer sur ordinateur · Sef** | player laptop, 6 moments, keyboard shortcuts |
| Mourir-Borin / Mort-Borin | **Mourir · la dernière soirée de Borin** | variant of session 3, laptop and phone, 7 moments |
| Tablette-MJ / Tablette-Soiree | **Tablette · mener la session 3 du canapé** | GM tablet (1180 × 820), 5 moments |

Components: GameCard, RadialGrid, Carte, Sprite, De, Gemme, Coeurs,
Cases, Horloge, Bouton, Objet, Etat, Perso, Plan.

## Generators

Run from the repository root; they rewrite files in `canvas/` and keep
any board layout already set in `canvas.json`.

- `player-evening-play.py` → `Jouer-Marc` (uses `player-evening.css`).
- `tv-evening.py` → `Ecran-TV`.
- `gm-evening-play.py` → `Mener-MJ`; `gm-evening-storyboard.py` → `Soiree-MJ`.
- `gm-prep-play.py` → `Preparer-MJ`; `gm-prep-storyboard.py` → `Preparation-MJ`.
- `gm-invite-play.py` → `Inviter-MJ`; `gm-invite-storyboard.py` → `Invitation-MJ`.
- `kit.py` holds what the boards below share: device frames (laptop,
  tablet, phone, TV), the side panel, `component()` for the moment
  logic, `board()` and `storyboard()` which write, scope and place a
  board. `avatar.py` is the layered pixel dwarf of the character creator.
- `player-create-play.py` → `Creer-Joueur`, `player-between-play.py` →
  `Entre-Joueur`, `gm-launch-play.py` → `Lancer-MJ`, `travel-play.py` →
  `Voyager-MJ`, `gm-rules-play.py` → `Regles-MJ`, `gm-map-play.py` →
  `Carte-MJ`, `player-desktop-play.py` → `Joueur-Ordi`, `death-play.py` →
  `Mourir-Borin`, `gm-tablet-play.py` → `Tablette-MJ`.
- `journey-storyboards.py` → the nine storyboards of those boards, and
  their layout on the canvas (board and storyboard side by side, three
  pairs per row). Run it after the board scripts; it overrides the
  position of these eighteen boards.
- `outdoor/*.py` → outdoor map PNGs; `items-prototype.py`,
  `walls-prototype.py`, `sprite-prototype.py` (+ `sprites.json`) →
  pixel-art previews.

Other boards were edited by hand. Regenerating `Jouer-Marc`,
`Ecran-TV`, `Mener-MJ`, `Preparer-MJ`, `Inviter-MJ` and the GM storyboards from these scripts reproduces the snapshot byte for byte.

## Publishing

Publish with the Artifact tool to the canvas URL above, root = the
folder holding `canvas/` contents. A publish is refused when a file
changed remotely since it was read: read it, merge, publish again.
Do not overwrite `canvas.json` if Romain changed the layout.
