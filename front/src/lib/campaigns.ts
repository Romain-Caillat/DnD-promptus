import { ApiError, apiRequest } from './api'

/** An NPC the players have met. */
interface PlayerNpc {
  id: string
  name: string
  title: string
  appearance: string
}

/** What players see of a campaign now: the server's single projection. */
interface PlayerView {
  title: string
  world: string
  playerHook: string
  scene: {
    id: string
    title: string
    readAloud: string
    place: { name: string; description: string } | null
    npcs: PlayerNpc[]
    opponents: { label: string; count: number }[]
  } | null
  clues: string[]
  npcs: PlayerNpc[]
}

export type PlayerViewResult =
  | { kind: 'ready'; view: PlayerView }
  | { kind: 'signed-out' }
  | { kind: 'not-found' }

/** `GET /api/campaigns/{id}/player-view`: the GM's preview of it. */
export async function fetchPlayerView(campaignId: string): Promise<PlayerViewResult> {
  try {
    const view = await apiRequest<PlayerView>(
      'GET',
      `/campaigns/${encodeURIComponent(campaignId)}/player-view`,
    )
    return { kind: 'ready', view }
  } catch (err) {
    if (err instanceof ApiError && err.status === 401) return { kind: 'signed-out' }
    if (err instanceof ApiError && err.status === 404) return { kind: 'not-found' }
    throw err
  }
}
