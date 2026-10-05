/**
 * A 12 × 12 item sprite (MEMORY.md §2: items are pixel art in colour):
 * rows of palette keys, `.` for transparent. The outline is added when
 * drawn. These are the sprites of the design canvas (« Objets et butin »,
 * `docs/design/items-prototype.py`); items generated for a campaign use
 * the same format.
 */
export interface PixelSprite {
  palette: Readonly<Record<string, string>>
  rows: readonly string[]
}

export const ITEM_SPRITES = {
  sword: {
    palette: { L: '#F2F2F2', M: '#A9B1BA', G: '#E0A82E', B: '#7A4A22' },
    rows: ['..........LL', '.........LLM', '........LLM.', '.......LLM..', '......LLM...', '.....LLM....', '..G.LLM.....', '...GLM......', '...BGG......', '..BB..G.....', '.BB.........', 'GB..........'],
  },
  axe: {
    palette: { L: '#E6E9EC', M: '#8E969F', B: '#8A5A2B', b: '#5C3A18', G: '#E0A82E' },
    rows: ['.M....Bb....', 'MLM...Bb....', 'MLLM..Bb....', 'MLLLMMBbM...', 'MLLLLLBbMM..', 'MLLLMMBbM...', 'MLLM..Bb....', 'MLM...Bb....', '.M....Bb....', '......Bb....', '......Bb....', '......GG....'],
  },
  potion: {
    palette: { C: '#8A5A2B', W: '#D8E4EA', G: '#9FB3BD', R: '#FF4D5E', r: '#C22A3C', h: '#FFFFFF' },
    rows: ['.....CC.....', '.....CC.....', '....WGGW....', '.....GG.....', '....WGGW....', '...WRRRRW...', '..WRhRRRRW..', '..WRhRRRRW..', '..WRRRRRrW..', '..WRRRRrrW..', '...WRRRrW...', '....WWWW....'],
  },
  shield: {
    palette: { M: '#A9B1BA', L: '#E6E9EC', B: '#8A5A2B', b: '#6A4220' },
    rows: ['.MMMMMMMMMM.', '.MBBBLLBBBM.', '.MBBBLLBBbM.', '.MBBBLLBBbM.', '.MLLLLLLLLM.', '.MBBBLLBBbM.', '.MBBBLLBBbM.', '..MBBLLBbM..', '..MBBLLBbM..', '...MBLLbM...', '....MLLM....', '.....MM.....'],
  },
  ring: {
    palette: { V: '#B28CFF', v: '#7A55D0', h: '#FFFFFF', G: '#F3CC63', g: '#B07A18' },
    rows: ['............', '.....VV.....', '....VhVV....', '....VVVv....', '.....GG.....', '...GG..GG...', '..G......G..', '..G......g..', '..g......g..', '...gg..gg...', '.....gg.....', '............'],
  },
  scroll: {
    palette: { P: '#EFE2C0', p: '#C9B48A', k: '#6E5A3A', R: '#C22A3C' },
    rows: ['............', '.pPPPPPPPPp.', 'pPPPPPPPPPPp', '.pPPPPPPPPp.', '..PkkkkkkP..', '..PPPPPPPP..', '..PkkkkPPP..', '..PPPPPPRR..', '..PkkkkPRR..', '.pPPPPPPPPp.', 'pPPPPPPPPPPp', '.pPPPPPPPPp.'],
  },
  crown: {
    palette: { G: '#F3CC63', g: '#B07A18', h: '#FFF7D6', R: '#FF4D5E', B: '#3D9BFF' },
    rows: ['............', '............', '.h...hh...h.', '.G...GG...G.', '.GG.GGGG.GG.', '.GGGGGGGGGG.', '.GhGGGGGGGG.', '.GRGGBBGGRG.', '.GGGGGGGGGG.', '.gggggggggg.', '............', '............'],
  },
  orb: {
    palette: { W: '#FFFFFF', A: '#BDF4FF', a: '#7FD8F0', h: '#FFFFFF', G: '#F3CC63', g: '#B07A18' },
    rows: ['............', '....WWWW....', '...WAAAAW...', '..WAhhAAAW..', '..WAhAAAAW..', '..WAAAAAaW..', '..WAAAAaaW..', '...WAAaaW...', '....WWWW....', '....gGGg....', '...gGGGGg...', '..gggggggg..'],
  },
  purse: {
    palette: { B: '#9A6A3A', b: '#6A4220', h: '#C99A6A', G: '#F3CC63', k: '#4A2E12' },
    rows: ['............', '....k..k....', '.....kk.....', '.....GG.....', '....BBBB....', '...BBhBBB...', '..BBhBBBBB..', '..BBBBBBbB..', '..BBBBBbbB..', '..BBBBbbbB..', '...BBBbbB...', '....BBBB....'],
  },
  chest: {
    palette: { B: '#9A6A3A', b: '#6A4220', G: '#E0A82E', k: '#2A1A08' },
    rows: ['.BBBBBBBBBB.', 'BbBBBBBBBBbB', 'BbBBBBBBBBbB', 'GGGGGGGGGGGG', 'BBBBBGGBBBBB', 'BbBBBGkBBBbB', 'BbBBBBBBBBbB', 'BbBBBBBBBBbB', 'BbBBBBBBBBbB', 'GGGGGGGGGGGG'],
  },
} as const satisfies Record<string, PixelSprite>

export type ItemSpriteName = keyof typeof ITEM_SPRITES
