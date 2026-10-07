import { useCallback, useEffect, useRef, useState, type ReactNode } from 'react'
import { useTranslation } from 'react-i18next'
import { useLiveChanges } from '@/features/live/useLiveChanges'
import { ApiError } from '@/lib/api'
import type { MediaList } from '@/lib/media'
import { fetchScreenMedia, fetchScreenView, pairingQrUrl, sayHello, type Hello, type ScreenView } from '@/lib/screens'
import { freshHighlights, type Highlight } from './focus'
import { TvStage } from './TvStage'
import './tv.css'

/** How often a TV waiting for its GM asks whether it was paired. */
export const PAIR_POLL_MS = 2_000
/** How long a big moment holds the screen. */
const HIGHLIGHT_MS = 6_000

type State = { kind: 'loading' } | { kind: 'error' } | { kind: 'pairing'; hello: Hello } | { kind: 'paired' }

const stateOf = (h: Hello): State => (h.paired ? { kind: 'paired' } : { kind: 'pairing', hello: h })

/**
 * `/tv` — the shared screen (session/pair-shared-screen, planche
 * « Lancer », moments 1 to 3). Opened on a TV, or on an old laptop
 * plugged into one, it shows a code and a QR; once the GM typed the code
 * it follows the evening by itself (tv/show-evening). Opened by the GM's
 * « Ouvrir la fenêtre TV », it is already paired: share that window on
 * Discord. Forgotten by the GM, it goes back to a code. Nothing here is
 * ever touched during the evening.
 */
export function TvPage() {
  const [state, setState] = useState<State>({ kind: 'loading' })
  const latest = useRef(0)

  const hello = useCallback(async () => {
    const request = ++latest.current
    let next: State | null
    try {
      next = stateOf(await sayHello())
    } catch {
      next = null
    }
    if (request !== latest.current) return
    // A failed hello keeps the code on screen; the next one retries.
    setState((s) => next ?? (s.kind === 'pairing' ? s : { kind: 'error' }))
  }, [])

  useEffect(() => {
    void hello()
  }, [hello])

  // Waiting for the GM: ask again, every few seconds (pairing takes
  // under 30 s, socket or not).
  useEffect(() => {
    if (state.kind !== 'pairing' && state.kind !== 'error') return
    const timer = window.setTimeout(() => void hello(), PAIR_POLL_MS)
    return () => window.clearTimeout(timer)
  }, [state, hello])

  if (state.kind === 'paired') return <PairedScreen onUnpaired={() => void hello()} />
  return (
    <main className="tv-root">
      <Scaled>
        {state.kind === 'pairing' && <Pairing hello={state.hello} />}
        {state.kind === 'loading' && <Centered textKey="tv.loading" />}
        {state.kind === 'error' && <Centered textKey="tv.error" />}
      </Scaled>
    </main>
  )
}

function Centered({ textKey }: { textKey: 'tv.loading' | 'tv.error' }) {
  const { t } = useTranslation()
  return (
    <div className="tv-center">
      <p className="text-[32px] text-mute-soft" role="status">
        {t(textKey)}
      </p>
    </div>
  )
}

function Pairing({ hello }: { hello: Hello }) {
  const { t } = useTranslation()
  const letters = Array.from(hello.code ?? '····')
  return (
    <div className="tv-center gap-10">
      <span className="type-label text-[20px] tracking-[0.3em]">{t('tv.brand')}</span>
      <div className="flex items-center gap-16">
        {hello.code && <img src={pairingQrUrl()} alt={t('tv.qr')} className="size-[300px] rounded-xl bg-ivory p-3" />}
        <div className="flex flex-col items-start gap-5">
          <span className="type-label text-[18px]">{t('tv.codeLabel')}</span>
          <div className="flex gap-3" aria-label={t('tv.code', { code: hello.code ?? '' })} role="img">
            {letters.map((c, i) => (
              <b key={i} className="tv-letter">
                {c}
              </b>
            ))}
          </div>
          <span className="max-w-[560px] text-left text-[22px] text-mute-soft">{t('tv.howTo')}</span>
        </div>
      </div>
    </div>
  )
}

/**
 * The 1920 × 1080 stage of the TV boards, scaled to the window so a TV,
 * a laptop or a Discord window all show the same picture.
 */
function Scaled({ children }: { children: ReactNode }) {
  const [scale, setScale] = useState(1)
  useEffect(() => {
    const fit = () => setScale(Math.min(window.innerWidth / 1920, window.innerHeight / 1080) || 1)
    fit()
    window.addEventListener('resize', fit)
    return () => window.removeEventListener('resize', fit)
  }, [])
  return (
    <div className="tv-stage" style={{ transform: `translate(-50%, -50%) scale(${scale})` }}>
      {children}
    </div>
  )
}

function PairedScreen({ onUnpaired }: { onUnpaired: () => void }) {
  const [shown, setShown] = useState<{ view: ScreenView; media: MediaList } | null>(null)
  const view = shown?.view ?? null
  const media = shown?.media ?? null
  const [highlight, setHighlight] = useState<Highlight | null>(null)
  const seen = useRef<Set<string>>(new Set())
  const latest = useRef(0)
  const gone = useRef(onUnpaired)
  useEffect(() => {
    gone.current = onUnpaired
  })

  const load = useCallback(async () => {
    const request = ++latest.current
    let next: { view: ScreenView; media: MediaList } | 'gone' | null
    try {
      const [v, m] = await Promise.all([fetchScreenView(), fetchScreenMedia()])
      next = { view: v, media: m }
    } catch (err) {
      next = err instanceof ApiError && err.status === 401 ? 'gone' : null
    }
    if (request !== latest.current) return
    // Forgotten by the GM: back to a code. Any other failure keeps what
    // is on screen; the next change retries.
    if (next === 'gone') gone.current()
    else setShown((s) => next ?? s)
  }, [])

  useEffect(() => {
    // `load` sets state only once its fetch has answered, never during
    // the effect; the compiler cannot see through the awaited pair.
    // eslint-disable-next-line react-hooks/set-state-in-effect
    void load()
  }, [load])

  useLiveChanges('', () => void load(), 'screen')

  // One big moment at a time, oldest first, each for a few seconds.
  useEffect(() => {
    if (highlight || !view) return
    const next = freshHighlights(view, seen.current)[0]
    if (next) setHighlight(next)
  }, [view, highlight])
  useEffect(() => {
    if (!highlight) return
    const timer = window.setTimeout(() => {
      seen.current.add(highlight.id)
      setHighlight(null)
    }, HIGHLIGHT_MS)
    return () => window.clearTimeout(timer)
  }, [highlight])

  return (
    <main className="tv-root">
      <Scaled>{view ? <TvStage view={view} media={media} highlight={highlight} /> : <Centered textKey="tv.loading" />}</Scaled>
    </main>
  )
}
