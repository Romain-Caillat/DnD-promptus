import { apiRequest } from './api'

/** How the HD-2D scene draws an element (`shared::elements::Form`). */
type ElementForm = 'texture' | 'flat' | 'facade' | 'character'

/** One element or figure a scene needs (`pixellab::Need`). */
interface Need {
  kind: 'element' | 'character'
  /** `<tileset>/<slot>`, or an NPC's id. */
  subject: string
  name: string
  form: ElementForm
  status: 'missing' | 'pending' | 'approved'
  asset: string | null
  generations: number
}

/** What a map still lacks from PixelLab (`pixellab::Needs`). */
export interface Needs {
  tileset: string | null
  palette: string[]
  needs: Need[]
  generations: number
}

/** campaign/import-pixellab-assets — what the map needs, and what fills it already. */
export function fetchNeeds(campaignId: string, map: string): Promise<Needs> {
  return apiRequest<Needs>('GET', `/campaigns/${encodeURIComponent(campaignId)}/elements?map=${encodeURIComponent(map)}`)
}
