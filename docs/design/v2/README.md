# Design v2 — mockups after the game bible

Static HTML boards, one per `design/*` ticket of `TICKETS.md`, built
from `docs/design/hd2d/quai.html` and `docs/bible.md`. They replace the
canvas boards of `docs/design/README.md` as the specification of the
game screens. Each board ends with a "Décidé" / "À trancher" box: the
decisions also go to `MEMORY.md` §0 and to the tickets they affect.

| Board | Ticket | State |
| --- | --- | --- |
| `cadre-pc.html` | `design/set-pc-game-frame` | validated 12 Oct 2026 (Octopath skin, party left, ZQSD) |
| `exploration.html` | `design/draw-explore-screen` | validated 12 Oct 2026 (names once known, perception announced, one speed) |
| `dialogue.html` | `design/draw-dialogue-screen` | drawn 12 Oct 2026, three questions open |
| `taverne-hd2d.html` | HD-2D from the key art | prototype 12 Oct 2026: `assets/maquette/a-salle.png` as a playable Three.js scene |

## Conventions

- One self-contained file per board (no shared CSS/JS): the screen is
  authored at 1920 × 1080 in `#screen`, scaled to the page by
  `layout()`; the board's controls sit above it, rules and decisions
  below.
- Octopath skin tokens (`--gold`, `--cream`, `.panel`, `.key`, `.mbtn`,
  `.toast`) are copied from `cadre-pc.html`; stat colours and shapes
  from `MEMORY.md` §2.
- Real campaign content only (`content/campaigns/corsaires/`): names,
  lines, checks and clues come from the scene being drawn.
- Sprites, portraits and the tavern are PixelLab images in `assets/`
  (`assets/README.md`: sheet layout, character ids, what was generated).

## Checking a board

```bash
node docs/design/v2/check.cjs   # page errors, overflow at 390 px, one screenshot per board
```

Look at the screenshots: the check finds errors, not overlaps.

There is no Node or Playwright on the Proxmox host. In PCT 105, with
Playwright installed under `/root/pw` (`bun add playwright` then
`bunx playwright install --with-deps chromium`):

```bash
pct exec 105 -- bash -lc 'cd /mnt/storage/DnD-promptus && NODE_PATH=/root/pw/node_modules bun docs/design/v2/check.cjs'
```
