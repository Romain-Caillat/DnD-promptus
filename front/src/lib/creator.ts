import type { CharacterLook } from '@/features/sprites/look'
import { apiRequest } from './api'
import type { CharacterSheet, CharacterView } from './play'

/** A piece of a sprite pack, as the creator lists it (`sprite::CataloguePiece`). */
export interface CataloguePiece {
  id: string
  /** In French, from the pack. */
  name: string
  /** Takes a dye from the `cloth` palette. */
  dyed: boolean
  /** Takes an accent from the `cloth` palette. */
  accented: boolean
}

export interface Swatch {
  id: string
  name: string
  colour: string
}

type Slot = 'body' | 'outfit' | 'armour' | 'hair' | 'beard' | 'headwear' | 'accessory' | 'weapon'

/** What a pack offers (`GET /api/sprites/packs/{pack}`). */
export interface PackCatalogue {
  id: string
  name: string
  slots: Record<Slot, CataloguePiece[]>
  palettes: { skin: Swatch[]; hair: Swatch[]; cloth: Swatch[] }
}

export interface Named {
  id: string
  name: string
  description: string
}

/** A class as the rules offer it (`projection::ClassView`). */
export interface ClassOption extends Named {
  primaryAbilities: string[]
  /** The class's own spread: where the abilities step starts. */
  abilities: Record<string, number>
  /** What that spread adds up to. Going over is flagged, never refused. */
  budget: number
  stats: CharacterView['stats']
}

/** What the creator offers in one campaign (`projection::CreationView`). */
export interface Creation {
  pack: string
  startLook: CharacterLook
  /** `null` when the server does not have the campaign's rule system. */
  rules: {
    name: string
    abilities: Named[]
    peoples: Named[]
    classes: ClassOption[]
  } | null
}

const play = (campaignId: string) => `/play/${encodeURIComponent(campaignId)}`

export function fetchCreation(campaignId: string): Promise<Creation> {
  return apiRequest<Creation>('GET', `${play(campaignId)}/creation`)
}

export function fetchPack(pack: string): Promise<PackCatalogue> {
  return apiRequest<PackCatalogue>('GET', `/sprites/packs/${encodeURIComponent(pack)}`)
}

/** Save the whole sheet as a draft; answers the character with its numbers. */
export function saveCharacter(campaignId: string, sheet: CharacterSheet): Promise<CharacterView> {
  return apiRequest<CharacterView>('PUT', `${play(campaignId)}/character`, sheet)
}

/** The co-GM writes the three answers up as a paragraph, for the player to keep or edit. */
export async function writeBackstory(
  campaignId: string,
  answers: { origin: string; loss: string; quest: string },
): Promise<string> {
  const r = await apiRequest<{ text: string }>('POST', `${play(campaignId)}/character/backstory`, answers)
  return r.text
}

/** Send the character to the GM. */
export function submitCharacter(campaignId: string): Promise<CharacterView> {
  return apiRequest<CharacterView>('POST', `${play(campaignId)}/character/submit`)
}

/** Where the creator lives in the app. */
export function creatorPath(campaignId: string): string {
  return `/partie/${encodeURIComponent(campaignId)}/personnage`
}

/** Points spent over the class's spread (0 when within it). */
export function overBudget(abilities: Record<string, number>, budget: number): number {
  const spent = Object.values(abilities).reduce((sum, v) => sum + v, 0)
  return Math.max(0, spent - budget)
}

/**
 * A look drawn at random from the pack, for « au hasard »: every piece
 * and colour comes from the catalogue, so the server always draws it.
 * Optional slots are left empty some of the time.
 */
export function randomLook(pack: PackCatalogue, random: () => number = Math.random): CharacterLook {
  const pick = <T>(list: readonly T[]): T | undefined =>
    list.length ? list[Math.floor(random() * list.length) % list.length] : undefined
  const maybe = <T>(odds: number, value: () => T | undefined): T | undefined =>
    random() < odds ? value() : undefined
  const cloth = () => pick(pack.palettes.cloth)?.id
  const worn = (slot: Slot) => {
    const piece = pick(pack.slots[slot] ?? [])
    if (!piece) return undefined
    return {
      piece: piece.id,
      dye: piece.dyed ? cloth() : undefined,
      accent: piece.accented ? cloth() : undefined,
    }
  }
  const accessory = maybe(0.3, () => worn('accessory'))
  return {
    pack: pack.id,
    body: pick(pack.slots.body)?.id ?? '',
    skin: pick(pack.palettes.skin)?.id ?? '',
    hair: { style: pick(pack.slots.hair ?? [])?.id, colour: pick(pack.palettes.hair)?.id ?? '' },
    beard: maybe(0.5, () => pick(pack.slots.beard ?? [])?.id),
    headwear: maybe(0.6, () => worn('headwear')),
    outfit: worn('outfit'),
    armour: maybe(0.25, () => worn('armour')),
    weapon: worn('weapon'),
    accessories: accessory ? [accessory] : undefined,
  }
}
