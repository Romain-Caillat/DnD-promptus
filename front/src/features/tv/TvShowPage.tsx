import { useCallback, useEffect, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Navigate, useParams } from 'react-router'
import { MusicPlayer } from '@/features/evening/MusicPlayer'
import { useLiveChanges } from '@/features/live/useLiveChanges'
import { MapTab } from '@/features/map/MapTab'
import { ApiError } from '@/lib/api'
import { fetchBoard } from '@/lib/board'
import { fetchEvening, type EveningView } from '@/lib/evening'
import { fetchPlayerMedia, imageOf, playerImageUrl, type MediaList } from '@/lib/media'
import { forgetTable } from '@/lib/tv'
import { cn } from '@/lib/utils'

interface Show {
  view: EveningView
  fight: boolean
  media: MediaList | null
}

type State = { kind: 'loading' } | { kind: 'error' } | { kind: 'gone' } | { kind: 'ready'; show: Show }

/** What the TV puts in front of the room, by priority. */
type Focus = 'reading' | 'fight' | 'scene' | 'lobby' | 'idle'

/** The focal moment of the evening, by priority. */
function focusOf({ view, fight }: Show): Focus {
  if (view.reading) return 'reading'
  if (fight) return 'fight'
  if (view.session?.status === 'live') return 'scene'
  if (view.session?.status === 'lobby') return 'lobby'
  return 'idle'
}

/**
 * `/tv/:campaignId` (tv/show-evening, board « Écran TV »): the shared
 * screen of the room, or the window the GM shares on Discord. It is a
 * spectator seat, so it shows only what the player projection gives a
 * spectator: the lobby, « Précédemment… » as the GM reads it, the scene
 * with its image, the fight on the map, the last line of the shared
 * journal, the music. It never acts.
 */
export function TvShowPage() {
  const { t } = useTranslation()
  const { campaignId = '' } = useParams()
  const [state, setState] = useState<State>({ kind: 'loading' })
  const [mapVersion, setMapVersion] = useState(0)
  const latest = useRef(0)

  const load = useCallback(async () => {
    const request = ++latest.current
    let next: Show | 'gone' | null
    try {
      const [view, board, media] = await Promise.all([
        fetchEvening(campaignId),
        fetchBoard(campaignId),
        fetchPlayerMedia(campaignId),
      ])
      next = { view, fight: Boolean(board?.fight?.live), media }
    } catch (err) {
      // Forgotten by the GM: back to a fresh code.
      next = err instanceof ApiError && err.status === 401 ? 'gone' : null
    }
    if (request !== latest.current) return
    if (next === 'gone') forgetTable()
    // A failed refetch keeps what is on screen; the next change retries.
    setState((s) =>
      next === 'gone' ? { kind: 'gone' } : next ? { kind: 'ready', show: next } : s.kind === 'ready' ? s : { kind: 'error' },
    )
  }, [campaignId])

  useEffect(() => {
    void load()
  }, [load])

  useLiveChanges(
    campaignId,
    (topics) => {
      if (topics.some((x) => ['world', 'story', 'session', 'map', 'fight'].includes(x))) void load()
      if (topics.some((x) => ['world', 'map', 'fight'].includes(x))) setMapVersion((v) => v + 1)
    },
    'player',
  )

  if (state.kind === 'gone') return <Navigate to="/tv" replace />
  if (state.kind !== 'ready') {
    return (
      <main className="surface-table grid min-h-dvh place-items-center p-8 text-chalk">
        {state.kind === 'loading' ? (
          <p role="status">{t('play.loading')}</p>
        ) : (
          <p role="alert">{t('play.error')}</p>
        )}
      </main>
    )
  }

  const { show } = state
  const { view } = show
  const focus = focusOf(show)
  const scene = view.campaign.scene
  const sceneImage = scene ? imageOf(show.media, 'scene', scene.id) : undefined
  const shared = view.journal.filter((l) => l.kind !== 'scene')
  const last = shared.length > 0 ? shared[shared.length - 1] : null

  return (
    <main className="surface-table relative flex min-h-dvh flex-col text-chalk" data-focus={focus}>
      <header className="flex items-baseline gap-4 px-10 pt-8">
        <h1 className="type-title text-[30px]">{view.campaign.title}</h1>
        {view.session && <span className="type-label">{t('tv.show.session', { number: view.session.number })}</span>}
        {view.music && (
          <div className="ml-auto w-64">
            <MusicPlayer music={view.music} />
          </div>
        )}
      </header>

      <section className="flex flex-1 flex-col justify-center gap-6 px-10 py-8">
        {focus === 'idle' && (
          <div className="flex flex-col items-center gap-4 text-center">
            <span className="type-title text-[64px] leading-none">{view.campaign.title}</span>
            <span className="text-body text-chalk-soft">{t('tv.show.idle')}</span>
          </div>
        )}
        {focus === 'lobby' && (
          <div className="flex flex-col items-center gap-6 text-center">
            <span className="type-label">{t('tv.show.tonight')}</span>
            <span className="type-title text-[72px] leading-none">{view.campaign.title}</span>
            <ul className="flex flex-wrap justify-center gap-8" aria-label={t('tv.show.here')}>
              {view.lobby.map((s) => (
                <li key={s.playerId} className="type-title text-[28px]">
                  {s.nickname}
                </li>
              ))}
            </ul>
            <span className="animate-pulse text-[22px] text-chalk-soft">{t('tv.show.waitingGm')}</span>
          </div>
        )}
        {focus === 'reading' && view.reading && (
          <div className="flex flex-col gap-6 px-[8%]">
            <h2 className="type-title text-[64px]">{t('evening.previously')}</h2>
            <ol className="flex flex-col gap-4" aria-label={t('evening.previously')}>
              {view.reading.lines.map((line, i) => (
                <li
                  key={i}
                  aria-hidden={i >= view.reading!.shown}
                  className={cn(
                    'type-narration text-[36px] leading-snug transition-opacity duration-700',
                    i < view.reading!.shown ? 'opacity-100' : 'opacity-0',
                  )}
                >
                  {line}
                </li>
              ))}
            </ol>
          </div>
        )}
        {focus === 'fight' && (
          <div className="mx-auto w-full max-w-5xl">
            <MapTab campaignId={campaignId} refreshKey={mapVersion} />
          </div>
        )}
        {focus === 'scene' &&
          (scene ? (
            <div className="grid items-center gap-8 lg:grid-cols-2">
              {sceneImage && (
                <img
                  src={playerImageUrl(campaignId, sceneImage.id)}
                  alt={scene.title}
                  className="w-full rounded-xl border border-line [image-rendering:pixelated]"
                />
              )}
              <div className="flex flex-col gap-4">
                <span className="type-label">{scene.place?.name ?? t('evening.scene')}</span>
                <h2 className="type-title text-[56px] leading-none">{scene.title}</h2>
                {scene.readAloud && <p className="type-narration text-[30px] leading-snug">{scene.readAloud}</p>}
              </div>
            </div>
          ) : (
            <p className="text-center text-[28px] text-chalk-soft">{t('evening.waitingScene')}</p>
          ))}
      </section>

      {last && focus !== 'reading' && (
        <footer className="flex items-baseline gap-4 border-t border-line px-10 py-5" aria-label={t('tv.show.feed')}>
          <span className="type-label">{t(`evening.journalKind.${last.kind}`)}</span>
          <span className="text-[22px]">{last.text}</span>
        </footer>
      )}
    </main>
  )
}
