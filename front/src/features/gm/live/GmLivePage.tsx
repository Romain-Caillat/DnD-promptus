import { useCallback, useEffect, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Link, Navigate, useParams } from 'react-router'
import { LiveIndicator } from '@/features/live/LiveIndicator'
import { useLiveChanges } from '@/features/live/useLiveChanges'
import { ApiError } from '@/lib/api'
import {
  editBoard,
  fetchGmBoard,
  giveLoot,
  gmFightCommand,
  showMap,
  startFight,
  type GmBoard,
} from '@/lib/board'
import {
  askCopilot,
  decide,
  dismissDraft,
  draftRecap,
  editRecap,
  endSession,
  fetchFeedback,
  fetchLiveScreen,
  giveSpotlight,
  openSession,
  playTrack,
  publishRecap,
  reveal,
  saveChanges,
  showDraft,
  startSession,
  writeNote,
  type LiveScreen,
} from '@/lib/evening'
import { chooseDate, fetchPlan, proposeDates, type Plan } from '@/lib/between'
import { askImage, decideImage, fetchGmMedia, type MediaList } from '@/lib/media'
import {
  createShop,
  deleteShop,
  fetchShops,
  openShop,
  revealLine,
  saveShop,
  type GmShops,
} from '@/lib/shop'
import { fetchScreens, forgetScreen, openWindow, pairScreen, setReading, type Screen } from '@/lib/tv'
import { PlanPanel, RecapPanel } from './BetweenPanels'
import { BoardPanel } from './BoardPanel'
import { CopilotPanel } from './CopilotPanel'
import { EndPanel, FeedbackPanel } from './EndPanel'
import { JournalPanel } from './JournalPanel'
import { LaunchPanel } from './LaunchPanel'
import { MediaPanel } from './MediaPanel'
import { RequestsPanel } from './RequestsPanel'
import { FactionsPanel } from './FactionsPanel'
import { ScenePanel } from './ScenePanel'
import { ShopsPanel } from './ShopsPanel'
import { TablePanel } from './TablePanel'
import { Btn } from './ui'

type PageState =
  | { kind: 'loading' }
  | { kind: 'signed-out' }
  | { kind: 'not-found' }
  | { kind: 'error' }
  | { kind: 'ready'; screen: LiveScreen }

/**
 * `/campagnes/:campaignId/soiree` — the GM runs the whole evening from
 * this one screen (gm/run-live-screen, planche « Mener »), on a laptop:
 * the table and its spotlight with the requests on the left; the scene,
 * the map and the fight in the middle; the co-GM, the journal and the
 * images on the right. One main action per moment in the header: open
 * the lobby, start, end. Each block refetches on its live topic.
 */
export function GmLivePage() {
  const { t } = useTranslation()
  const { campaignId = '' } = useParams()
  const [state, setState] = useState<PageState>({ kind: 'loading' })
  const [board, setBoard] = useState<GmBoard | null>(null)
  const [media, setMedia] = useState<MediaList | null>(null)
  const [plan, setPlan] = useState<Plan | null>(null)
  const [screens, setScreens] = useState<Screen[]>([])
  const [shops, setShops] = useState<GmShops | null>(null)
  const [error, setError] = useState<string | null>(null)
  const latest = useRef(0)
  const latestBoard = useRef(0)
  const latestMedia = useRef(0)
  const latestPlan = useRef(0)
  const latestScreens = useRef(0)
  const latestShops = useRef(0)

  const loadScreen = useCallback(async () => {
    const request = ++latest.current
    let next: PageState
    try {
      next = { kind: 'ready', screen: await fetchLiveScreen(campaignId) }
    } catch (err) {
      next =
        err instanceof ApiError && err.status === 401
          ? { kind: 'signed-out' }
          : err instanceof ApiError && err.status === 404
            ? { kind: 'not-found' }
            : { kind: 'error' }
    }
    if (request !== latest.current) return
    // A failed refetch keeps what is on screen; the next change retries.
    setState((s) => (next.kind === 'error' && s.kind === 'ready' ? s : next))
  }, [campaignId])

  const loadBoard = useCallback(async () => {
    const request = ++latestBoard.current
    let next: GmBoard | null
    try {
      next = await fetchGmBoard(campaignId)
    } catch {
      next = null
    }
    if (request !== latestBoard.current) return
    // A failed refetch keeps what is on screen; the next change retries.
    setBoard((b) => next ?? b)
  }, [campaignId])

  const loadMedia = useCallback(async () => {
    const request = ++latestMedia.current
    let next: MediaList | null
    try {
      next = await fetchGmMedia(campaignId)
    } catch {
      next = null
    }
    if (request !== latestMedia.current) return
    setMedia((m) => next ?? m)
  }, [campaignId])

  const loadPlan = useCallback(async () => {
    const request = ++latestPlan.current
    let next: { plan: Plan | null } | null
    try {
      next = { plan: await fetchPlan(campaignId) }
    } catch {
      next = null
    }
    if (request !== latestPlan.current) return
    // A failed refetch keeps what is on screen; the next change retries.
    setPlan((p) => (next ? next.plan : p))
  }, [campaignId])

  const loadScreens = useCallback(async () => {
    const request = ++latestScreens.current
    let next: Screen[] | null
    try {
      next = await fetchScreens(campaignId)
    } catch {
      next = null
    }
    if (request !== latestScreens.current) return
    setScreens((s) => next ?? s)
  }, [campaignId])

  const loadShops = useCallback(async () => {
    const request = ++latestShops.current
    let next: GmShops | null
    try {
      next = await fetchShops(campaignId)
    } catch {
      next = null
    }
    if (request !== latestShops.current) return
    setShops((s) => next ?? s)
  }, [campaignId])

  useEffect(() => {
    void loadShops()
    void loadScreen()
    void loadBoard()
    void loadMedia()
    void loadPlan()
    void loadScreens()
  }, [loadScreen, loadBoard, loadMedia, loadPlan, loadScreens, loadShops])

  const live = useLiveChanges(campaignId, (topics) => {
    if (topics.some((x) => ['session', 'world', 'story', 'table', 'desk'].includes(x) || x.startsWith('character:'))) {
      void loadScreen()
    }
    if (topics.includes('session')) {
      void loadPlan()
      void loadShops()
    }
    if (topics.includes('table')) void loadScreens()
    if (topics.some((x) => ['map', 'fight', 'world'].includes(x))) void loadBoard()
    if (topics.includes('desk')) void loadMedia()
  })

  /** Run a gesture; refresh what it moved; say what went wrong. */
  async function act<T>(call: () => Promise<T>, after?: (v: T) => void): Promise<T | null> {
    setError(null)
    try {
      const v = await call()
      after?.(v)
      void loadScreen()
      return v
    } catch (err) {
      setError(err instanceof ApiError ? err.code : 'UNEXPECTED')
      return null
    }
  }

  if (state.kind === 'signed-out') return <Navigate to="/connexion" replace />
  if (state.kind !== 'ready') {
    return (
      <main className="surface-table flex min-h-dvh flex-col gap-4 p-6 text-chalk">
        {state.kind === 'loading' && <p role="status">{t('gmLive.loading')}</p>}
        {state.kind === 'not-found' && <p role="alert">{t('gmLive.notFound')}</p>}
        {state.kind === 'error' && <p role="alert">{t('gmLive.error')}</p>}
      </main>
    )
  }

  const { screen } = state
  const session = screen.session
  const base = `/campagnes/${encodeURIComponent(campaignId)}`
  const boardGesture = (call: () => Promise<GmBoard>) => act(call, setBoard)

  return (
    <main className="surface-table flex min-h-dvh flex-col gap-3 p-4 text-chalk">
      <header className="flex flex-wrap items-center justify-between gap-3">
        <div className="flex flex-col">
          <Link className="text-caption text-mute-soft underline underline-offset-4" to={base}>
            {t('gmLive.back')}
          </Link>
          <h1 className="type-title text-heading">
            {session ? t('gmLive.session', { number: session.number, status: t(`gmLive.status.${session.status}`) }) : t('gmLive.noSession')}
          </h1>
        </div>
        <div className="flex items-center gap-3">
          <LiveIndicator status={live.status} />
          <span className="text-caption text-mute-soft">{t('live.players', { count: live.presence.players.length })}</span>
          {!session && (
            <Btn main onClick={() => void act(() => openSession(campaignId))}>
              {t('gmLive.open')}
            </Btn>
          )}
          {session?.status === 'lobby' && (
            <Btn main onClick={() => void act(() => startSession(campaignId))}>
              {t('gmLive.start')}
            </Btn>
          )}
        </div>
      </header>
      {error && (
        <p role="alert" className="rounded-button border border-stat-atk px-3 py-2 text-body">
          {t(`gmLive.errors.${error}`, { defaultValue: t('gmLive.errors.UNEXPECTED', { code: error }) })}
        </p>
      )}
      <div className="grid gap-3 lg:grid-cols-[minmax(260px,1fr)_minmax(420px,2fr)_minmax(280px,1fr)]">
        <div className="flex flex-col gap-3">
          <TablePanel screen={screen} onSpotlight={(p) => void act(() => giveSpotlight(campaignId, p))} />
          {session?.status === 'live' && (
            <RequestsPanel screen={screen} onDecide={(id, d) => void act(() => decide(campaignId, id, d))} />
          )}
        </div>
        <div className="flex flex-col gap-3">
          {(!session || session.status === 'lobby' || session.readingLine !== null) && (
            <LaunchPanel
              screen={screen}
              screens={screens}
              onPair={async (code) => (await act(() => pairScreen(campaignId, code), setScreens)) !== null}
              onWindow={() => {
                // Opened in the click, or the browser blocks it; its address follows.
                const tab = window.open('', '_blank')
                void act(
                  () => openWindow(campaignId),
                  (secret) => {
                    if (tab) tab.location.href = `/tv?cle=${encodeURIComponent(secret)}`
                    void loadScreens()
                  },
                )
              }}
              onForget={(id) => void act(() => forgetScreen(campaignId, id), () => void loadScreens())}
              onReading={(line) => void act(() => setReading(campaignId, line))}
            />
          )}
          {session && (
            <ScenePanel
              screen={screen}
              onReveal={(r) => void act(() => reveal(campaignId, r))}
              onTrack={(i) => void act(() => playTrack(campaignId, i))}
            />
          )}
          {board && (
            <BoardPanel
              campaignId={campaignId}
              data={board}
              media={media}
              live={session?.status === 'live'}
              onShow={(m) => void boardGesture(() => showMap(campaignId, m))}
              onEdit={(e) => void boardGesture(() => editBoard(campaignId, e))}
              onStart={(n) => void boardGesture(() => startFight(campaignId, n))}
              onCommand={(c) => void boardGesture(() => gmFightCommand(campaignId, c))}
              onLoot={(index, character) => void boardGesture(() => giveLoot(campaignId, [{ index, character }]))}
            />
          )}
          {!session && screen.lastEnded && (
            <RecapPanel
              key={screen.lastEnded.id}
              session={screen.lastEnded}
              aiConfigured={screen.ai.configured}
              onSave={async (text) => {
                await act(() => editRecap(campaignId, screen.lastEnded!.id, text))
              }}
              onPublish={async (text) => {
                await act(async () => {
                  await editRecap(campaignId, screen.lastEnded!.id, text)
                  return publishRecap(campaignId, screen.lastEnded!.id)
                })
              }}
              onDraft={() => act(() => draftRecap(campaignId, screen.lastEnded!.id))}
            />
          )}
          {!session && screen.lastEnded && (
            <FeedbackPanel
              load={() => fetchFeedback(campaignId, screen.lastEnded!.id)}
              onChanges={async (text) => {
                await act(() => saveChanges(campaignId, screen.lastEnded!.id, text))
              }}
            />
          )}
        </div>
        <div className="flex flex-col gap-3">
          {session && (
            <CopilotPanel
              screen={screen}
              onAsk={async (kind, prompt, npc) => {
                await act(() => askCopilot(campaignId, kind, prompt, npc))
              }}
              onShow={(d, n, l) => void act(() => showDraft(campaignId, d, n, l))}
              onDismiss={(d) => void act(() => dismissDraft(campaignId, d))}
              onReveal={(r) => void act(() => reveal(campaignId, r))}
            />
          )}
          {!session && (
            <PlanPanel
              plan={plan}
              screen={screen}
              onPropose={async (options, minutes) => {
                await act(() => proposeDates(campaignId, options, minutes), setPlan)
              }}
              onChoose={async (at) => {
                await act(() => chooseDate(campaignId, at), setPlan)
              }}
            />
          )}
          {session && (
            <FactionsPanel
              screen={screen}
              live={session.status === 'live'}
              onReveal={(r) => void act(() => reveal(campaignId, r))}
            />
          )}
          {shops && (
            <ShopsPanel
              data={shops}
              onSave={async (id, input) =>
                (await act(() => (id ? saveShop(campaignId, id, input) : createShop(campaignId, input)), () => void loadShops())) !==
                null
              }
              onOpen={(shop, open) => void act(() => openShop(campaignId, shop, open), () => void loadShops())}
              onReveal={(shop, line) => void act(() => revealLine(campaignId, shop, line), () => void loadShops())}
              onDelete={(shop) => void act(() => deleteShop(campaignId, shop), () => void loadShops())}
            />
          )}
          <JournalPanel
            screen={screen}
            onNote={async (kind, text, shared) => {
              await act(() => writeNote(campaignId, kind, text, shared))
            }}
          />
          <MediaPanel
            campaignId={campaignId}
            screen={screen}
            media={media}
            onAsk={async (kind, subject, direction) => {
              await act(() => askImage(campaignId, kind, subject, direction), () => void loadMedia())
            }}
            onDecide={(a, approve) => void act(() => decideImage(campaignId, a, approve), () => void loadMedia())}
          />
          {session && session.status !== 'ended' && (
            <EndPanel
              screen={screen}
              onEnd={async (recap, previously) => {
                await act(() => endSession(campaignId, recap, previously))
              }}
              onDraft={() => act(() => draftRecap(campaignId, session.id))}
            />
          )}
        </div>
      </div>
    </main>
  )
}
