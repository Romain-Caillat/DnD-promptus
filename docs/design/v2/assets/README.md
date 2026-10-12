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

The feet sit at about (44, 79) in a cell. The boards draw them at 1.5×.

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
