import { apiRequest } from './api'
import type { OutcomeBand, RollBreakdown } from './rules'

// --- The player's side (`projection::trade`) ------------------------------------

interface LineView {
  key: string
  name: string
  description: string
  price: number
  /** With the player's won haggle, while they still have it. */
  discountedPrice: number | null
  /** Units left; `null` for as many as wanted. */
  stock: number | null
}

interface HaggleTerms {
  ability: string
  abilityName: string
  difficulty: number
  label: string | null
  discountPercent: number
  mine: { band: OutcomeBand; outcome: string; roll: RollBreakdown; discountLeft: boolean } | null
}

export interface ShopView {
  id: string
  name: string
  keeper: string
  abbr: string
  lines: LineView[]
  haggle: HaggleTerms | null
  surcharge: number
}

/** Open shops, the player's purse, who they can give to. */
export interface TradeView {
  purse: { name: string; abbr: string; amount: number } | null
  shops: ShopView[]
  companions: { id: string; name: string }[]
}

const play = (campaignId: string) => `/play/${encodeURIComponent(campaignId)}`

export function fetchTrade(campaignId: string): Promise<TradeView> {
  return apiRequest<TradeView>('GET', `${play(campaignId)}/trade`)
}

/** One unit of `line`; `discounted` spends the won haggle on it. */
export function buy(campaignId: string, shop: string, line: string, discounted: boolean): Promise<TradeView> {
  return apiRequest<TradeView>('POST', `${play(campaignId)}/shops/${encodeURIComponent(shop)}/buy`, {
    line,
    discounted,
  })
}

/** The one haggle at `shop`: the server rolls. */
export function haggle(campaignId: string, shop: string): Promise<TradeView> {
  return apiRequest<TradeView>('POST', `${play(campaignId)}/shops/${encodeURIComponent(shop)}/haggle`)
}

/** Hand some of a bag line, or an amount of money, to a companion. */
export function give(
  campaignId: string,
  gift: { to: string; entry: string; qty: number } | { to: string; amount: number },
): Promise<TradeView> {
  return apiRequest<TradeView>('POST', `${play(campaignId)}/character/give`, gift)
}

// --- The GM's screen (`api::trade`) ---------------------------------------------

export interface ShopLine {
  key: string
  item?: string
  storyItem?: string
  name?: string
  description?: string
  price: number
  stock?: number | null
  hidden: boolean
}

export interface Haggle {
  ability: string
  difficulty: number
  discountPercent: number
  fumbleSurcharge: number
  criticalReveals: boolean
}

export interface GmShop {
  id: string
  name: string
  keeper: string
  npc: string | null
  open: boolean
  currency: string
  lines: (ShopLine & { displayName: string })[]
  haggle: Haggle | null
  surcharge: number
  haggles: { characterId: string; name: string; outcome: string; total: number; discountLeft: boolean }[]
}

export interface ShopsScreen {
  shops: GmShop[]
  /** What a shop may sell: the rules' items and the story's. */
  sellable: { kind: 'rules' | 'story'; id: string; name: string; price: number | null }[]
  /** NPCs of the story who keep a shop. */
  keepers: { id: string; name: string; title: string }[]
  /** `null` while the rules declare no resource to pay with. */
  currency: { id: string; name: string; abbr: string } | null
  abilities: { id: string; name: string }[] | null
  difficulties: { name: string; value: number }[] | null
}

const gm = (campaignId: string) => `/campaigns/${encodeURIComponent(campaignId)}/shops`

export function fetchShops(campaignId: string): Promise<ShopsScreen> {
  return apiRequest<ShopsScreen>('GET', gm(campaignId))
}

export function createShop(
  campaignId: string,
  from: { fromNpc: string } | { name: string; keeper: string },
): Promise<ShopsScreen> {
  return apiRequest<ShopsScreen>('POST', gm(campaignId), from)
}

export function saveShop(
  campaignId: string,
  shop: string,
  edit: { name: string; keeper: string; lines: ShopLine[]; haggle: Haggle | null },
): Promise<ShopsScreen> {
  return apiRequest<ShopsScreen>('PUT', `${gm(campaignId)}/${encodeURIComponent(shop)}`, edit)
}

export function setShopOpen(campaignId: string, shop: string, open: boolean): Promise<ShopsScreen> {
  return apiRequest<ShopsScreen>('POST', `${gm(campaignId)}/${encodeURIComponent(shop)}/open`, { open })
}

export function revealLine(campaignId: string, shop: string, line: string): Promise<ShopsScreen> {
  return apiRequest<ShopsScreen>('POST', `${gm(campaignId)}/${encodeURIComponent(shop)}/reveal`, { line })
}

export function deleteShop(campaignId: string, shop: string): Promise<ShopsScreen> {
  return apiRequest<ShopsScreen>('DELETE', `${gm(campaignId)}/${encodeURIComponent(shop)}`)
}
