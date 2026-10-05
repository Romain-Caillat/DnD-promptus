/**
 * The six rarity tiers of skill cards and items (MEMORY.md §2), in the
 * order of `Rarity` in `shared/src/story/model.rs`: the strings are its
 * serde names, so item data maps onto them one to one. A tier is read
 * from the material and from 1 to 6 diamonds, never from a hue.
 */
export const RARITIES = ['common', 'uncommon', 'rare', 'epic', 'legendary', 'divine'] as const
export type Rarity = (typeof RARITIES)[number]

/** How many diamonds a tier shows: 1 for common, 6 for divine. */
export function rarityRank(rarity: Rarity): number {
  return RARITIES.indexOf(rarity) + 1
}

/** The card materials of `styles/tokens.css`, as literal class names. */
export const RARITY_MATERIAL: Record<Rarity, string> = {
  common: 'material-common',
  uncommon: 'material-uncommon',
  rare: 'material-rare',
  epic: 'material-epic',
  legendary: 'material-legendary',
  divine: 'material-divine',
}
