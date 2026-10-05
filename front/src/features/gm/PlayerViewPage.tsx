import { useCallback, useEffect, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Navigate, useParams } from 'react-router'
import { LiveIndicator } from '@/features/live/LiveIndicator'
import { useLiveChanges } from '@/features/live/useLiveChanges'
import { fetchPlayerView, type PlayerViewResult } from '@/lib/campaigns'

type State = { kind: 'loading' } | { kind: 'error' } | PlayerViewResult

/** The topics the player view is built from. */
const VIEW_TOPICS = ['world', 'story']

/**
 * `/campagnes/:campaignId/vue-joueurs` — what the players see right now,
 * as the server projects it, kept up to date by the live channel: the GM
 * can leave it open next to their prep, on a laptop or a phone.
 */
export function PlayerViewPage() {
  const { t } = useTranslation()
  const { campaignId = '' } = useParams()
  const [state, setState] = useState<State>({ kind: 'loading' })
  // Only the latest request may write: a slow answer to an older one
  // must not overwrite a newer state.
  const latest = useRef(0)

  const load = useCallback(async () => {
    const request = ++latest.current
    let next: State
    try {
      next = await fetchPlayerView(campaignId)
    } catch {
      next = { kind: 'error' }
    }
    if (request === latest.current) setState(next)
  }, [campaignId])

  useEffect(() => {
    void load()
    return () => {
      latest.current += 1
    }
  }, [load])

  const live = useLiveChanges(campaignId, (topics) => {
    if (topics.some((topic) => VIEW_TOPICS.includes(topic))) void load()
  })

  if (state.kind === 'signed-out') return <Navigate to="/connexion" replace />

  return (
    <main className="mx-auto flex min-h-dvh max-w-2xl flex-col gap-6 p-6">
      <header className="flex flex-wrap items-center justify-between gap-3">
        <p className="text-sm text-muted-foreground">{t('gm.playerView.kicker')}</p>
        <div className="flex flex-wrap items-center gap-3">
          <p className="text-xs text-muted-foreground">
            {t('live.players', { count: live.presence.players.length })}
          </p>
          <LiveIndicator status={live.status} />
        </div>
      </header>
      {state.kind === 'loading' && <p role="status">{t('gm.home.loading')}</p>}
      {state.kind === 'error' && <p role="alert">{t('auth.errors.generic')}</p>}
      {state.kind === 'not-found' && <p role="alert">{t('gm.playerView.notFound')}</p>}
      {state.kind === 'ready' && (
        <article className="flex flex-col gap-6">
          <div>
            <h1 className="text-2xl font-semibold tracking-tight">{state.view.title}</h1>
            {state.view.world && <p className="text-sm text-muted-foreground">{state.view.world}</p>}
          </div>
          {state.view.playerHook && <p>{state.view.playerHook}</p>}
          <section aria-labelledby="pv-scene" className="flex flex-col gap-2">
            <h2 id="pv-scene" className="text-lg font-semibold">
              {t('gm.playerView.scene')}
            </h2>
            {state.view.scene ? (
              <>
                <p className="font-semibold">{state.view.scene.title}</p>
                {state.view.scene.place && (
                  <p className="text-sm text-muted-foreground">{state.view.scene.place.name}</p>
                )}
                {state.view.scene.readAloud && <p className="italic">{state.view.scene.readAloud}</p>}
              </>
            ) : (
              <p className="text-muted-foreground">{t('gm.playerView.noScene')}</p>
            )}
          </section>
          <section aria-labelledby="pv-clues" className="flex flex-col gap-2">
            <h2 id="pv-clues" className="text-lg font-semibold">
              {t('gm.playerView.clues')}
            </h2>
            {state.view.clues.length === 0 ? (
              <p className="text-muted-foreground">{t('gm.playerView.noClues')}</p>
            ) : (
              <ul className="list-disc pl-5">
                {state.view.clues.map((clue) => (
                  <li key={clue}>{clue}</li>
                ))}
              </ul>
            )}
          </section>
          <section aria-labelledby="pv-npcs" className="flex flex-col gap-2">
            <h2 id="pv-npcs" className="text-lg font-semibold">
              {t('gm.playerView.npcs')}
            </h2>
            {state.view.npcs.length === 0 ? (
              <p className="text-muted-foreground">{t('gm.playerView.noNpcs')}</p>
            ) : (
              <ul className="flex flex-col gap-1">
                {state.view.npcs.map((npc) => (
                  <li key={npc.id}>
                    <span className="font-semibold">{npc.name}</span>
                    {npc.title && <span className="text-muted-foreground"> · {npc.title}</span>}
                  </li>
                ))}
              </ul>
            )}
          </section>
        </article>
      )}
    </main>
  )
}
