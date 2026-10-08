import { useCallback, useEffect, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { CardButton } from '@/components/game/CardButton'
import { FacetedDie, facesOf } from '@/components/game/FacetedDie'
import { Kbd } from '@/components/game/Kbd'
import { RollDetail } from '@/components/game/RollDetail'
import { Toast, ToastStack } from '@/components/game/Toast'
import { ApiError } from '@/lib/api'
import {
  answerFeedback,
  arrive,
  askGm,
  fetchEvening,
  followRequest,
  rollRequest,
  type Answer,
  type Card,
  type EveningView,
  type JournalKind,
  type RequestView,
  type SceneCard,
} from '@/lib/evening'
import { fetchPlayerMedia, imageOf, playerImageUrl, type MediaList } from '@/lib/media'
import type { ModifierSource } from '@/lib/rules'
import { cardIndex, cardKey, useShortcuts } from '@/lib/useShortcuts'
import { cn } from '@/lib/utils'
import { EveningEnd } from '@/features/play/between/EveningEnd'
import { useBetween } from '@/features/play/between/useBetween'
import { Market } from '@/features/play/trade/Market'
import type { PlayerTab } from '@/lib/between'
import { TravelMoments } from '@/features/travel/PlayerTravel'
import { MusicPlayer } from './MusicPlayer'
import { NextSession } from './NextSession'

type State = { kind: 'loading' } | { kind: 'error' } | { kind: 'ready'; view: EveningView }

/** Journal kinds that drop a toast on the phone (player/receive-rewards). */
const TOASTED = new Set<JournalKind>(['clue', 'npc', 'loot', 'item', 'narration', 'fight'])

/**
 * The Game tab (player/play-scene, planche « Jouer »): the lobby before
 * the session, then the scene — place, text read aloud, its image — the
 * music, the hand of cards to ask the GM, the die to roll when the GM
 * asks for a check, and the answer. After the session, « Précédemment… »
 * and the three questions. What happens to the group drops as toasts.
 * Everything comes from the server's projection; nothing is decided here.
 * On a computer (player/play-on-desktop) the keyboard plays too: a digit
 * picks a card, Space rolls the die the GM asked for.
 */
export function GameTab({
  campaignId,
  refreshKey,
  seated,
  onTab,
  keyboard = false,
}: {
  campaignId: string
  refreshKey: number
  /** A player with a character in play (a spectator only watches). */
  seated: boolean
  /** Opens another tab of the player's page (between sessions). */
  onTab?: (tab: PlayerTab) => void
  /** The keys are this tab's: shortcuts on, their hints shown. */
  keyboard?: boolean
}) {
  const { t } = useTranslation()
  const [state, setState] = useState<State>({ kind: 'loading' })
  const [media, setMedia] = useState<MediaList | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [toasts, setToasts] = useState<{ key: string; kind: JournalKind; text: string }[]>([])
  const seen = useRef<Set<string> | null>(null)
  const latest = useRef(0)

  const show = useCallback((view: EveningView) => {
    const keys = view.journal.map((l) => `${l.createdAt}|${l.text}`)
    if (seen.current) {
      const fresh = view.journal.filter((l) => TOASTED.has(l.kind) && !seen.current!.has(`${l.createdAt}|${l.text}`))
      if (fresh.length > 0) {
        setToasts((ts) => [...ts, ...fresh.map((l) => ({ key: `${l.createdAt}|${l.text}`, kind: l.kind, text: l.text }))].slice(-4))
      }
    }
    seen.current = new Set(keys)
    setState({ kind: 'ready', view })
  }, [])

  const load = useCallback(async () => {
    const request = ++latest.current
    let next: [EveningView, MediaList] | null
    try {
      next = await Promise.all([fetchEvening(campaignId), fetchPlayerMedia(campaignId)])
    } catch {
      next = null
    }
    if (request !== latest.current) return
    // A failed refetch keeps what is on screen; the next change retries.
    setState((s) => (next ? s : s.kind === 'ready' ? s : { kind: 'error' }))
    setMedia((m) => next?.[1] ?? m)
    if (next) show(next[0])
  }, [campaignId, show])

  useEffect(() => {
    void load()
  }, [load, refreshKey])

  async function act(call: () => Promise<EveningView>) {
    setError(null)
    try {
      show(await call())
    } catch (err) {
      setError(err instanceof ApiError ? err.code : 'UNEXPECTED')
    }
  }

  // Space rolls the oldest die the GM asked of me.
  const toRoll =
    state.kind === 'ready' && state.view.session?.status === 'live'
      ? state.view.requests.find((r) => r.status === 'check' && r.check)
      : undefined
  useShortcuts(keyboard && Boolean(toRoll), (key) => {
    if (key !== ' ' || !toRoll) return false
    void act(() => rollRequest(campaignId, toRoll.id))
    return true
  })

  if (state.kind === 'loading') return <p role="status">{t('play.loading')}</p>
  if (state.kind === 'error') return <p role="alert">{t('play.error')}</p>
  const { view } = state
  const session = view.session
  const scene = view.campaign.scene
  const sceneImage = scene ? imageOf(media, 'scene', scene.id) : undefined
  const intro = scene ? imageOf(media, 'intro', scene.act) : undefined

  return (
    <div className="flex flex-col gap-4">
      <ToastStack className="fixed inset-x-3 top-3 z-20 mx-auto max-w-md">
        {toasts.map((x) => (
          <Toast
            key={x.key}
            tone={x.kind === 'loot' || x.kind === 'item' ? 'good' : 'info'}
            kicker={t(`evening.journalKind.${x.kind}`)}
            onDismiss={() => setToasts((ts) => ts.filter((y) => y.key !== x.key))}
          >
            {x.text}
          </Toast>
        ))}
      </ToastStack>
      {error && (
        <p role="alert" className="rounded-button border border-stat-atk px-3 py-2 text-body">
          {t(`evening.errors.${error}`, { defaultValue: t('evening.errors.UNEXPECTED') })}
        </p>
      )}
      <Market campaignId={campaignId} refreshKey={refreshKey} seated={seated} />
      {!session && (
        <NoSession
          view={view}
          campaignId={campaignId}
          seated={seated}
          refreshKey={refreshKey}
          onView={show}
          onTab={onTab}
        />
      )}
      {session?.status === 'lobby' && (
        <Lobby view={view} seated={seated} onArrive={(soundOk) => act(() => arrive(campaignId, soundOk, true))} />
      )}
      {session?.status === 'live' && view.launch && <Launch launch={view.launch} />}
      {session?.status === 'live' && !view.launch && (
        <>
          {view.music && <MusicPlayer music={view.music} />}
          <TravelMoments campaignId={campaignId} refreshKey={refreshKey} />
          {scene ? (
            <section className="flex flex-col gap-2">
              {intro && (
                <video
                  src={playerImageUrl(campaignId, intro.id)}
                  controls
                  playsInline
                  preload="metadata"
                  aria-label={t('evening.intro')}
                  className="w-full rounded-xl border border-line"
                />
              )}
              {sceneImage && (
                <img
                  src={playerImageUrl(campaignId, sceneImage.id)}
                  alt={scene.title}
                  className="w-full rounded-xl border border-line [image-rendering:pixelated]"
                />
              )}
              <span className="type-label">{scene.place?.name ?? t('evening.scene')}</span>
              <h2 className="type-title text-[24px]">{scene.title}</h2>
              {scene.readAloud && (
                <p className="type-narration text-[18px] leading-snug text-chalk-soft">{scene.readAloud}</p>
              )}
              {scene.npcs.length > 0 && (
                <p className="text-caption text-mute-soft">
                  {t('evening.here', { names: scene.npcs.map((n) => n.name).join(', ') })}
                </p>
              )}
            </section>
          ) : (
            <p className="text-body text-chalk-soft">{t('evening.waitingScene')}</p>
          )}
          {view.campaign.clues.length > 0 && (
            <section className="flex flex-col gap-1.5">
              <h3 className="type-label">{t('evening.clues')}</h3>
              <ul className="flex flex-col gap-1.5">
                {view.campaign.clues.map((c) => (
                  <li key={c} className="rounded-button bg-ivory px-3 py-2 text-body text-ink shadow-ivory-flat">
                    {c}
                  </li>
                ))}
              </ul>
            </section>
          )}
          <Requests
            view={view}
            onRoll={(id) => act(() => rollRequest(campaignId, id))}
            onFollow={(id, f) => act(() => followRequest(campaignId, id, f))}
            keyboard={keyboard}
          />
          {seated && (
            <Hand
              cards={view.cards}
              keyboard={keyboard}
              onAsk={(card, text) => act(() => askGm(campaignId, card, text))}
            />
          )}
        </>
      )}
    </div>
  )
}

/**
 * gm/launch-session: « Précédemment… » arrives on the phone sentence by
 * sentence, as the GM reads it; the newest one is the bright one. The
 * scene follows when the GM sends it.
 */
function Launch({ launch }: { launch: NonNullable<EveningView['launch']> }) {
  const { t } = useTranslation()
  return (
    <section className="flex flex-col gap-3" aria-label={t('evening.previously')}>
      <span className="type-label">{t('evening.previously')}</span>
      <ol className="flex flex-col gap-2">
        {launch.lines.map((line, i) => (
          <li
            key={`${i}-${line}`}
            className={cn(
              'type-narration text-[22px] leading-snug motion-safe:animate-pop',
              i === launch.lines.length - 1 ? 'text-chalk' : 'text-chalk-soft',
            )}
          >
            {line}
          </li>
        ))}
      </ol>
      {launch.lines.length < launch.total && <p className="text-caption text-mute">{t('evening.launchWait')}</p>}
    </section>
  )
}

/**
 * Between two sessions (player/play-between-sessions): the end of the
 * last evening, the way to level up, « Précédemment… » and the
 * chronicle, then the three questions of the feedback.
 */
function NoSession({
  view,
  campaignId,
  seated,
  refreshKey,
  onView,
  onTab,
}: {
  view: EveningView
  campaignId: string
  seated: boolean
  refreshKey: number
  onView: (v: EveningView) => void
  onTab?: (tab: PlayerTab) => void
}) {
  const { t } = useTranslation()
  const between = useBetween(campaignId, refreshKey)
  const go = (tab: PlayerTab) => onTab?.(tab)
  return (
    <section className="flex flex-col gap-3">
      <p className="text-body text-chalk-soft">{t('evening.noSession')}</p>
      <NextSession campaignId={campaignId} refreshKey={refreshKey} />
      {between.kind === 'ready' && between.between.last ? (
        <EveningEnd
          between={between.between}
          onLevelUp={() => go('perso')}
          onChronicle={() => go('journal')}
          onSheet={() => go('perso')}
        />
      ) : (
        view.previously && (
          <div className="surface-slab flex flex-col gap-1 p-3.5">
            <span className="type-label">{t('evening.previously')}</span>
            <p className="type-narration text-[18px] leading-snug">{view.previously}</p>
          </div>
        )
      )}
      {seated && view.feedback && !view.feedback.answered && (
        <Feedback number={view.feedback.number} onSend={async (a) => onView(await answerFeedback(campaignId, a))} />
      )}
      {view.feedback?.answered && <p className="text-caption text-mute">{t('evening.feedback.thanks')}</p>}
    </section>
  )
}

function Lobby({ view, seated, onArrive }: { view: EveningView; seated: boolean; onArrive: (soundOk: boolean) => void }) {
  const { t } = useTranslation()
  const [heard, setHeard] = useState(false)
  return (
    <section className="flex flex-col gap-3">
      <h2 className="type-title text-[24px]">{t('evening.lobby.title', { number: view.session?.number })}</h2>
      {view.previously && (
        <div className="surface-slab flex flex-col gap-1 p-3.5">
          <span className="type-label">{t('evening.previously')}</span>
          <p className="type-narration text-[18px] leading-snug">{view.previously}</p>
        </div>
      )}
      <ul className="flex flex-wrap gap-1.5" aria-label={t('evening.lobby.here')}>
        {view.lobby.map((s) => (
          <li key={s.playerId} className="rounded-full border border-line px-2.5 py-1 text-caption">
            {s.nickname}
            {s.soundOk ? ` ${t('evening.lobby.soundMark')}` : ''}
          </li>
        ))}
      </ul>
      {seated && (
        <>
          <button
            type="button"
            className="pixel-key pixel-key-dark pixel-cursor self-start py-1.5 pr-3 pb-2.5 text-label [--px:2px]"
            onClick={() => {
              beep()
              setHeard(true)
            }}
          >
            {t('evening.lobby.testSound')}
          </button>
          <CardButton
            title={t('evening.lobby.arrive')}
            subtitle={heard ? t('evening.lobby.soundOk') : t('evening.lobby.soundUnknown')}
            onClick={() => onArrive(heard)}
          />
        </>
      )}
      <p className="text-caption text-mute">{t('evening.lobby.wait')}</p>
    </section>
  )
}

/** A short tone: the sound check of the lobby. */
function beep() {
  try {
    const ctx = new AudioContext()
    const osc = ctx.createOscillator()
    osc.frequency.value = 660
    osc.connect(ctx.destination)
    osc.start()
    osc.stop(ctx.currentTime + 0.25)
  } catch {
    // No audio on this device: the player says so by not confirming.
  }
}

const OTHER: Card = { kind: 'other' }

function cardOf(c: SceneCard): Card {
  return c.kind === 'ability' ? { kind: 'ability', ability: c.id } : { kind: 'action', action: c.id }
}

/**
 * The cards a player plays to ask the GM, and « Autre… ». With the
 * keyboard, 1 to 9 pick the cards in order, 0 « Autre… », Escape puts
 * the card back, and Enter sends what is written (Shift+Enter: a line).
 */
function Hand({
  cards,
  keyboard,
  onAsk,
}: {
  cards: SceneCard[]
  keyboard: boolean
  onAsk: (card: Card, text: string) => void
}) {
  const { t } = useTranslation()
  const [picked, setPicked] = useState<Card | null>(null)
  const [text, setText] = useState('')
  const name = (c: Card) =>
    c.kind === 'other'
      ? t('evening.hand.other')
      : (cards.find((x) => x.kind === c.kind && x.id === (c.kind === 'ability' ? c.ability : c.action))?.name ?? '')
  const hand = [...cards.map(cardOf), OTHER]
  useShortcuts(keyboard, (key) => {
    if (key === 'Escape' && picked) {
      setPicked(null)
      return true
    }
    const index = key === '0' ? hand.length - 1 : cardIndex(key)
    if (index === null || index >= cards.length + (key === '0' ? 1 : 0)) return false
    setPicked(hand[index])
    return true
  })
  function send() {
    if (!picked) return
    onAsk(picked, text.trim())
    setPicked(null)
    setText('')
  }
  return (
    <section className="flex flex-col gap-2">
      <h3 className="type-label">{t('evening.hand.title')}</h3>
      <div className="flex gap-2 overflow-x-auto pb-1">
        {hand.map((c, i) => {
          const label = name(c)
          const shortcut = c.kind === 'other' ? '0' : cardKey(i)
          const card = cards.find((x) => x.kind === c.kind && x.id === (c.kind === 'ability' ? c.ability : c.kind === 'action' ? c.action : ''))
          const on = picked !== null && JSON.stringify(picked) === JSON.stringify(c)
          return (
            <button
              key={JSON.stringify(c)}
              type="button"
              aria-pressed={on}
              title={card?.description}
              className={cn(
                'pixel-choice flex min-w-24 flex-none flex-col items-start gap-0.5 py-2 pr-3 pb-2.5 text-left',
              )}
              onClick={() => setPicked(on ? null : c)}
            >
              <span className="text-body font-bold">{label}</span>
              {card?.modifier !== null && card?.modifier !== undefined && (
                <span className="text-caption tabular-nums">{t('evening.hand.modifier', { value: card.modifier })}</span>
              )}
              {keyboard && shortcut && <Kbd>{shortcut}</Kbd>}
            </button>
          )
        })}
      </div>
      {picked && (
        <form
          className="flex flex-col gap-2"
          onSubmit={(e) => {
            e.preventDefault()
            send()
          }}
        >
          <textarea
            value={text}
            autoFocus={keyboard}
            onChange={(e) => setText(e.target.value)}
            onKeyDown={(e) => {
              if (keyboard && e.key === 'Enter' && !e.shiftKey) {
                e.preventDefault()
                send()
              }
            }}
            maxLength={500}
            rows={2}
            placeholder={t('evening.hand.placeholder')}
            aria-label={t('evening.hand.what', { card: name(picked) })}
            className="pixel-field p-2 text-body"
          />
          <CardButton type="submit" title={t('evening.hand.send', { card: name(picked) })} />
        </form>
      )}
    </section>
  )
}

function Requests({
  view,
  onRoll,
  onFollow,
  keyboard,
}: {
  view: EveningView
  onRoll: (id: string) => void
  onFollow: (id: string, f: 'withdraw' | 'contest') => void
  keyboard: boolean
}) {
  const { t } = useTranslation()
  const mine = view.requests.filter((r) => r.status !== 'withdrawn').slice(-4).reverse()
  if (mine.length === 0) return null
  return (
    <section className="flex flex-col gap-2">
      <h3 className="type-label">{t('evening.requests.title')}</h3>
      {mine.map((r) => (
        <RequestCard key={r.id} r={r} view={view} onRoll={onRoll} onFollow={onFollow} keyboard={keyboard} />
      ))}
    </section>
  )
}

function RequestCard({
  r,
  view,
  onRoll,
  onFollow,
  keyboard,
}: {
  r: RequestView
  view: EveningView
  onRoll: (id: string) => void
  onFollow: (id: string, f: 'withdraw' | 'contest') => void
  keyboard: boolean
}) {
  const { t } = useTranslation()
  const abilityName = (id: string) => view.cards.find((c) => c.kind === 'ability' && c.id === id)?.name ?? id
  const sourceName = (s: ModifierSource) => ('id' in s ? abilityName(s.id) : t('evening.requests.longRange'))
  return (
    <article className="surface-slab flex flex-col gap-2 p-3" data-status={r.status}>
      <div className="flex items-baseline justify-between gap-2">
        <span className="text-body font-bold">{r.card.name ?? t('evening.hand.other')}</span>
        <span className="type-label">{t(`evening.requests.status.${r.status}`)}</span>
      </div>
      {r.text && <p className="text-caption text-chalk-soft">{r.text}</p>}
      {r.gmReason && <p className="rounded-button bg-ivory px-3 py-2 text-body text-ink">{r.gmReason}</p>}
      {r.status === 'check' && r.check && (
        <CardButton
          title={t('evening.requests.roll')}
          subtitle={[
            t('evening.requests.against', {
              ability: r.check.abilityName,
              difficulty: r.check.label ?? r.check.difficulty,
            }),
            keyboard ? t('evening.requests.spaceToo') : null,
          ]
            .filter(Boolean)
            .join(' · ')}
          onClick={() => onRoll(r.id)}
        />
      )}
      {r.roll && (
        <div className="flex items-center gap-3">
          <FacetedDie key={r.id} faces={facesOf(r.roll.die)} value={r.roll.natural} />
          <div className="flex flex-col">
            {r.outcome && <span className="type-title text-[16px]">{r.outcome}</span>}
          </div>
        </div>
      )}
      {r.roll && (
        <RollDetail
          roll={r.roll}
          bandName={(b) => (b === r.roll?.band && r.outcome ? r.outcome : t(`evening.band.${b}`))}
          sourceName={sourceName}
          targetName={r.check?.label ?? undefined}
        />
      )}
      <div className="flex gap-2">
        {r.status === 'pending' && (
          <button type="button" className="text-caption underline" onClick={() => onFollow(r.id, 'withdraw')}>
            {t('evening.requests.withdraw')}
          </button>
        )}
        {(r.status === 'refused' || r.status === 'rolled') && !r.contested && (
          <button type="button" className="text-caption underline" onClick={() => onFollow(r.id, 'contest')}>
            {t('evening.requests.contest')}
          </button>
        )}
        {r.contested && <span className="text-caption text-mute">{t('evening.requests.contested')}</span>}
      </div>
    </article>
  )
}

const ANSWERS: Answer[] = ['yes', 'partly', 'no']
const QUESTIONS = ['rulesClear', 'hadMoment', 'knowsNext'] as const

/** session/collect-player-feedback: three taps and a word. */
function Feedback({
  number,
  onSend,
}: {
  number: number
  onSend: (a: { rulesClear: Answer; hadMoment: Answer; knowsNext: Answer; comment: string }) => Promise<void>
}) {
  const { t } = useTranslation()
  const [answers, setAnswers] = useState<Partial<Record<(typeof QUESTIONS)[number], Answer>>>({})
  const [comment, setComment] = useState('')
  const complete = QUESTIONS.every((q) => answers[q])
  return (
    <form
      className="surface-slab flex flex-col gap-3 p-3.5"
      onSubmit={(e) => {
        e.preventDefault()
        if (!complete) return
        void onSend({
          rulesClear: answers.rulesClear!,
          hadMoment: answers.hadMoment!,
          knowsNext: answers.knowsNext!,
          comment: comment.trim(),
        })
      }}
    >
      <span className="type-label">{t('evening.feedback.title', { number })}</span>
      {QUESTIONS.map((q) => (
        <fieldset key={q} className="flex flex-col gap-1.5">
          <legend className="text-body font-bold">{t(`evening.feedback.${q}`)}</legend>
          <div className="flex gap-1.5">
            {ANSWERS.map((a) => (
              <button
                key={a}
                type="button"
                aria-pressed={answers[q] === a}
                className={cn(
                  'pixel-choice flex-1 py-2 pr-2 pb-2.5 text-body',
                )}
                onClick={() => setAnswers((s) => ({ ...s, [q]: a }))}
              >
                {t(`evening.feedback.answer.${a}`)}
              </button>
            ))}
          </div>
        </fieldset>
      ))}
      <textarea
        value={comment}
        onChange={(e) => setComment(e.target.value)}
        rows={2}
        maxLength={500}
        aria-label={t('evening.feedback.comment')}
        placeholder={t('evening.feedback.comment')}
        className="pixel-field p-2 text-body"
      />
      <CardButton type="submit" disabled={!complete} title={t('evening.feedback.send')} />
    </form>
  )
}
