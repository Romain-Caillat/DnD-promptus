import { useCallback, useEffect, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Panel } from '@/features/gm/live/ui'
import { LiveIndicator } from '@/features/live/LiveIndicator'
import type { LiveState } from '@/features/live/useLiveChanges'
import { MapCanvas } from '@/features/map/MapCanvas'
import { playerMarks } from '@/features/map/pins'
import { HexMap } from '@/features/travel/HexMap'
import { ApiError } from '@/lib/api'
import { fetchPlayerView, type PlayerView } from '@/lib/campaigns'
import { fetchPlayersMap, type PlayersMap as PlayersBoard } from '@/lib/maps'
import type { Tileset } from '@/lib/media'
import type { Scene } from '@/features/map/render'

type MapState = { kind: 'loading' } | { kind: 'error'; code: string } | ({ kind: 'ready' } & PlayersBoard)

/**
 * maps/preview-as-player — the map as the players would receive it now,
 * from the server's own projection (the route the TV's board comes
 * from): fog, NPCs not met, clues not found, scene numbers and Claude's
 * proposals are simply not in what it sends. Nothing is filtered here —
 * what the route returns is drawn, and nothing else. `reload` follows
 * the live topics.
 */
export function usePlayersMap(campaignId: string, mapId: string | null, on: boolean) {
  // Each answer names its map: switching maps shows « loading » until the new one lands.
  const [state, setState] = useState<{ of: string; state: MapState } | null>(null)
  const latest = useRef(0)
  const load = useCallback(async () => {
    if (!on || !mapId) return
    const request = ++latest.current
    let next: MapState
    try {
      next = { kind: 'ready', ...(await fetchPlayersMap(campaignId, mapId)) }
    } catch (err) {
      next = { kind: 'error', code: err instanceof ApiError ? err.code : 'action' }
    }
    if (request === latest.current) setState({ of: mapId, state: next })
  }, [campaignId, mapId, on])
  useEffect(() => {
    void load()
    return () => {
      latest.current += 1
    }
  }, [load])
  const shown: MapState = state && state.of === mapId ? state.state : { kind: 'loading' }
  return { state: shown, reload: load }
}

/** The map as the players' screens draw it: their tokens, the pins they visited and the clues they found. */
export function PlayersMap({
  state,
  tileset,
  atlases,
  backdrop,
  touch,
}: {
  state: MapState
  tileset: Tileset | null
  atlases: Scene['atlases']
  backdrop: Scene['backdrop']
  touch: boolean
}) {
  const { t } = useTranslation()
  if (state.kind === 'loading') return <p role="status">{t('prep.loading')}</p>
  if (state.kind === 'error') {
    return (
      <p role="alert" className="text-body text-mute-soft">
        {t(`atlas.players.errors.${state.code}`, { defaultValue: t('atlas.players.errors.action') })}
      </p>
    )
  }
  const { board } = state
  const pins = playerMarks(board.pins, board.clues)
  return (
    <div className="flex min-w-0 flex-col gap-2" data-testid="players-map">
      <p className="text-caption text-mute-soft">{state.shown ? t('atlas.players.shown') : t('atlas.players.ahead')}</p>
      {board.map.scale === 'world' ? (
        <HexMap
          map={board.map}
          tileset={tileset}
          party={board.tokens[0]?.at ?? null}
          pins={pins}
          className="max-h-[70dvh]"
        />
      ) : (
        <MapCanvas
          scene={{ map: board.map, tileset, atlases, backdrop, tokens: board.tokens }}
          fightEvents={board.fight?.events}
          pins={pins}
          touch={touch}
          className="max-h-[70dvh]"
        />
      )}
    </div>
  )
}

/**
 * Beside the players' map, what their screens say now (the player
 * view, `GET …/player-view`): the campaign, its hook, the scene open,
 * the clues found and the characters met — with the live channel's
 * state and how many players are connected, so the GM can leave it open
 * next to the table.
 */
export function PlayersSection({ campaignId, version, live }: { campaignId: string; version: number; live: LiveState }) {
  const { t } = useTranslation()
  const [view, setView] = useState<PlayerView | null | 'error'>(null)
  useEffect(() => {
    let live = true
    fetchPlayerView(campaignId).then(
      (r) => {
        if (live) setView(r.kind === 'ready' ? r.view : 'error')
      },
      () => {
        if (live) setView('error')
      },
    )
    return () => {
      live = false
    }
  }, [campaignId, version])

  return (
    <Panel title={t('gm.playerView.kicker')}>
      <div className="flex flex-wrap items-center gap-3">
        <p className="text-caption text-mute-soft">{t('live.players', { count: live.presence.players.length })}</p>
        <LiveIndicator status={live.status} />
      </div>
      {view === null && <p role="status">{t('prep.loading')}</p>}
      {view === 'error' && <p role="alert">{t('prep.error')}</p>}
      {view && view !== 'error' && (
        <article className="flex flex-col gap-3">
          <div>
            <h3 className="type-title text-[16px]">{view.title}</h3>
            {view.world && <p className="text-caption text-mute-soft">{view.world}</p>}
          </div>
          {view.playerHook && <p className="text-body">{view.playerHook}</p>}
          <section aria-labelledby="pv-scene" className="flex flex-col gap-1">
            <h3 id="pv-scene" className="type-label text-mute-soft">
              {t('gm.playerView.scene')}
            </h3>
            {view.scene ? (
              <>
                <p className="font-semibold">{view.scene.title}</p>
                {view.scene.place && <p className="text-caption text-mute-soft">{view.scene.place.name}</p>}
                {view.scene.readAloud && <p className="italic">{view.scene.readAloud}</p>}
              </>
            ) : (
              <p className="text-caption text-mute-soft">{t('gm.playerView.noScene')}</p>
            )}
          </section>
          <section aria-labelledby="pv-clues" className="flex flex-col gap-1">
            <h3 id="pv-clues" className="type-label text-mute-soft">
              {t('gm.playerView.clues')}
            </h3>
            {view.clues.length === 0 ? (
              <p className="text-caption text-mute-soft">{t('gm.playerView.noClues')}</p>
            ) : (
              <ul className="list-disc pl-5">
                {view.clues.map((clue) => (
                  <li key={clue}>{clue}</li>
                ))}
              </ul>
            )}
          </section>
          <section aria-labelledby="pv-npcs" className="flex flex-col gap-1">
            <h3 id="pv-npcs" className="type-label text-mute-soft">
              {t('gm.playerView.npcs')}
            </h3>
            {view.npcs.length === 0 ? (
              <p className="text-caption text-mute-soft">{t('gm.playerView.noNpcs')}</p>
            ) : (
              <ul className="flex flex-col gap-1">
                {view.npcs.map((npc) => (
                  <li key={npc.id}>
                    <span className="font-semibold">{npc.name}</span>
                    {npc.title && <span className="text-mute-soft"> · {npc.title}</span>}
                  </li>
                ))}
              </ul>
            )}
          </section>
        </article>
      )}
    </Panel>
  )
}
