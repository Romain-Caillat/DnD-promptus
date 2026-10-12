# Assets of the v2 boards — PixelLab, 12 October 2026

Generated with the PixelLab MCP (Tier 1 subscription), 178 generations.
Content from `content/campaigns/corsaires/campagne.yaml` (NPC `appearance`
and `portrait`, `art` of `sc_perroquet`, theme `port-1718`). The four
players have no appearance in the campaign: theirs follow their class and
the colours the boards already used.

## `sprites/<id>.png` — characters

Standard mode, side view, 4 directions, ~64 px on a 92 px canvas, then
packed into one sheet per character (8 columns of 92 × 92 cells):

| Row | Content |
| --- | --- |
| 0 | still: south, east, north, west |
| 1–4 | `breathing-idle` (4 frames), south, east, north, west |
| 5–8 | `walking-8-frames` (8 frames), south, east, north, west |

The feet sit at about (44, 79) in a cell. The boards draw every image of
the room at 2.5× (one PixelLab pixel = 2.5 px of the 1920 × 1080 screen),
so characters, props and walls share one pixel size.

| Sheet | Who | PixelLab character id |
| --- | --- | --- |
| `yann` | Yann · Bretteur | `e8ed9706-5afd-42ee-a5b8-ac78242cc5cf` |
| `lea` | Léa · Vigie | `5a6ba9ce-d70c-4f75-9489-605ab28a7376` |
| `max` | Max · Canonnier | `cf0274bc-83c6-495a-b257-0a75331db581` |
| `romain` | Romain · Navigateur | `6d25283b-973a-455f-9141-636c5a50653d` |
| `goulven` | Mère Goulven (`pnj_goulven`) | `2f73a3a8-fd23-4472-a120-9af2536a1a27` |
| `jacquot` | Jacquot le Sourd (`pnj_jacquot`) | `fe29a7f7-3c2b-4778-b84a-b984c2a52f53` |
| `rene` | Petit René (`pnj_rene`) | `74e46d3f-936d-46b7-9d55-74834d67c937` |
| `fanch` | Fañch le Beau (`pnj_fanch`) | `83a45c2b-e877-4048-835a-afe9384adcb9` |
| `anais` | Anaïs (`pnj_anais`) | `05e73227-7498-49d5-928d-5a88b5a4ee8e` |

Gaps with the campaign text: Mère Goulven is not "massive" and has no
ladle; Jacquot wears a cap instead of a balding skull.

## `portraits/` — busts, 128 × 128

`character_to_portrait` from each sprite (15 generations each), so a bust
keeps its sprite's face and clothes. Mère Goulven's `mefiante` and `ravie`
are edits of `goulven-neutre` (`edit_image_pro_flash`, 6 generations each).

## `taverne/` — the Perroquet Noyé

Props seen from the front (`create_map_object`, side view, 1 generation
each): `comptoir`, `etagere`, `table`, `fenetre`, `chandelle`, `perroquet`
(second try: the first was a bright, healthy parrot). Textures
(`create_image_pixflux`, 1 generation each): `sol` (floor planks, tiled)
and `mur` (second try: the first had an electric bulb). The bottom 12 px
of `mur.png` are paving that the floor hides.

## `maquette/` — key art of the tavern (`create_image_pro`, 688 × 384)

`a-salle.png` (25 generations) is the reference picture chosen by Romain:
the tavern full, every character of `sc_perroquet` and the four players.
`b-comptoir.png` (25) is the closer variant that was not kept (René twice).
`a-vide.png` (25) is the same room emptied, made with `a-salle.png` as
reference and style: it is the backdrop of `taverne-hd2d.html`.

`persos-joueurs.png` and `persos-pnj.png` (25 each, plus 25 for a first NPC
sheet cut at the head) are the nine characters redrawn in the key art's
style, as lineups facing the viewer; `perso-<id>.png` are the cut-outs, the
feet the lineups cropped extended by hand (last row repeated, a dark sole).

## `hd2d/` — the characters of `taverne-hd2d.html`

Players: PixelLab v3 characters rotated from `perso-<id>.png` (2–3
generations each), then `walking-8-frames` in 4 directions (1 per
direction). Rows: `still` (south, east, north, west) and `walk_<dir>`.
NPCs face the viewer only: `animate_image` on `perso-<id>.png`, 4 frames
plus the input (2 generations each), one `idle` row. Cell sizes vary;
`meta.json` gives each sheet's cell, rows and feet.

| Who | v3 character id |
| --- | --- |
| Yann | `e40ffbad-82b1-4a60-8b75-4a4c3313a709` |
| Léa | `f86e7420-8753-45d1-8c10-e9e3d467aed1` |
| Max | `72ad587f-5c3b-4c66-a0f1-f585005c3c4d` |
| Romain | `844163e7-7997-427f-8da4-1221222db592` |

The walk is the steadier `skeleton-v3` one (`walk2` on PixelLab), except
Yann south and Léa west, which keep the first template walk (in its frames
5–8 the figure shrinks a little). Every row is set on its own lowest pixel,
because the `skeleton-v3` canvas places the figure higher than the stills.
The players have no idle animation: generation was stopped on 12 October.
The NPC lineup cropped Goulven's, Fañch's and Anaïs's heads flat at the
top: their frames got a few rows added by hand above the flat top, each
narrower than the one below and outlined, so the head is round again.

## `quai/` — an abandoned try

`quai.png` (25) and `quai-vide.png` (25, its "customs" sign repainted
"DOUANES" by hand) were the second level of the recipe. Dropped: the
street runs diagonally to a vanishing point, which breaks movement and
would distort the combat grid (`MEMORY.md` §0). The crew lineup made for
it (25) was never downloaded.

`hd2d-data.js` carries the backdrop and these sheets as data URLs, because
WebGL refuses images read from `file://`. Rebuild it after changing any of
them: `python3 docs/design/v2/assets/hd2d/build-data.py`.
