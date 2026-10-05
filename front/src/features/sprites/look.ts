/**
 * A character's look: pieces of a pack and their colours, never an
 * image (`shared/src/sprite/look.rs` is the reference). The server draws
 * it — one renderer for the phone, the GM screen and the TV.
 */
type Worn = string | { piece: string; dye?: string; accent?: string }

export interface CharacterLook {
  pack: string
  body: string
  skin: string
  hair: { style?: string; colour: string }
  beard?: string
  headwear?: Worn
  outfit?: Worn
  armour?: Worn
  weapon?: Worn
  accessories?: Worn[]
}

/** Heroes face east (right), enemies west (left). */
export type Facing = 'east' | 'west'

/**
 * The effect a condition shows on the character (rule system,
 * `conditions[].visual`).
 */
export const CONDITION_VISUALS = [
  'poison',
  'etourdi',
  'endormi',
  'invisible',
  'feu',
  'entrave',
  'effraye',
  'beni',
  'ko',
  'cible',
] as const
export type ConditionVisual = (typeof CONDITION_VISUALS)[number]

/** The rendered sprite, in sprite pixels: the 20 x 26 grid of the starter packs plus a 1-px margin for the outline. */
export const SPRITE_WIDTH = 22
export const SPRITE_HEIGHT = 28

function worn(w: Worn | undefined) {
  if (w === undefined) return undefined
  if (typeof w === 'string') return { piece: w }
  return { piece: w.piece, dye: w.dye, accent: w.accent }
}

/**
 * The URL of a look's PNG. Keys are written in one fixed order, so the
 * same look is always the same URL and the browser cache holds it once.
 */
export function spriteUrl(look: CharacterLook, facing: Facing = 'east'): string {
  const canonical = {
    pack: look.pack,
    body: look.body,
    skin: look.skin,
    hair: { style: look.hair.style, colour: look.hair.colour },
    beard: look.beard,
    headwear: worn(look.headwear),
    outfit: worn(look.outfit),
    armour: worn(look.armour),
    weapon: worn(look.weapon),
    accessories: look.accessories?.map(worn),
  }
  const params = new URLSearchParams({ look: JSON.stringify(canonical), facing })
  return `/api/sprites/render.png?${params.toString()}`
}
