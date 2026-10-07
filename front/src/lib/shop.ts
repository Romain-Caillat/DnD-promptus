import { apiRequest } from './api'
import type { CharacterView } from './play'
import type { RollBreakdown } from './rules'

const play = (campaignId: string) => `/play/${encodeURIComponent(campaignId)}`
const gm = (campaignId: string) => `/campaigns/${encodeURIComponent(campaignId)}`

/** The open shop on a player's phone (`projection::shop::ShopView`). */
export interface ShopView {
  name: string
  lines: {
    id: string
    name: string
    description: string
    /** What it costs me, my haggle counted. */
    price: number
    listPrice: number
    stock: number | null
    /** Whether my haggle on it won, once tried. */
    haggled: boolean | null
  }[]
  haggle: { ability: string; abilityName: string; difficulty: number; discount: number } | null
  purse: { name: string; abbr: string; amount: number } | null
}

export interface Friend {
  id: string
  name: string
  nickname: string
}

export type Gift = { to: string; coins: number } | { to: string; entry: string; qty: number }

export function fetchShop(campaignId: string): Promise<ShopView | null> {
  return apiRequest<ShopView | null>('GET', `${play(campaignId)}/shop`)
}

export function buy(campaignId: string, line: string, qty = 1): Promise<ShopView | null> {
  return apiRequest<ShopView | null>('POST', `${play(campaignId)}/shop/buy`, { line, qty })
}

export function haggle(campaignId: string, line: string): Promise<{ roll: RollBreakdown; shop: ShopView | null }> {
  return apiRequest('POST', `${play(campaignId)}/shop/haggle`, { line })
}

export function fetchParty(campaignId: string): Promise<Friend[]> {
  return apiRequest<Friend[]>('GET', `${play(campaignId)}/party`)
}

export function give(campaignId: string, gift: Gift): Promise<CharacterView> {
  return apiRequest<CharacterView>('POST', `${play(campaignId)}/character/give`, gift)
}

/** One line of a shop, as the GM writes it. */
interface ShopLine {
  id: string
  item: string | null
  name: string
  description: string
  price: number
  stock: number | null
  hidden: boolean
}

interface HaggleRule {
  ability: string
  difficulty: number
  discount: number
}

/** A shop as the GM sees it (`shops::Shop`). */
export interface Shop {
  id: string
  name: string
  open: boolean
  lines: ShopLine[]
  haggle: HaggleRule | null
  haggles: { line: string; player: string; success: boolean }[]
}

export interface ShopInput {
  name: string
  lines: (Omit<ShopLine, 'id'> & { id?: string })[]
  haggle: HaggleRule | null
}

export interface GmShops {
  shops: Shop[]
  items: { id: string; name: string; price: number | null }[]
  abilities: [string, string][]
  currency: { name: string; abbr: string } | null
}

export function fetchShops(campaignId: string): Promise<GmShops> {
  return apiRequest<GmShops>('GET', `${gm(campaignId)}/shops`)
}

export function createShop(campaignId: string, input: ShopInput): Promise<Shop> {
  return apiRequest<Shop>('POST', `${gm(campaignId)}/shops`, input)
}

export function saveShop(campaignId: string, shop: string, input: ShopInput): Promise<Shop> {
  return apiRequest<Shop>('PUT', `${gm(campaignId)}/shops/${encodeURIComponent(shop)}`, input)
}

export function deleteShop(campaignId: string, shop: string): Promise<void> {
  return apiRequest<void>('DELETE', `${gm(campaignId)}/shops/${encodeURIComponent(shop)}`)
}

export function openShop(campaignId: string, shop: string, open: boolean): Promise<Shop> {
  return apiRequest<Shop>('POST', `${gm(campaignId)}/shops/${encodeURIComponent(shop)}/open`, { open })
}

export function revealLine(campaignId: string, shop: string, line: string): Promise<Shop> {
  return apiRequest<Shop>('POST', `${gm(campaignId)}/shops/${encodeURIComponent(shop)}/reveal`, { line })
}
