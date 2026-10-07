import { useEffect, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import type { CopilotAction, CopilotKind, Draft, LiveScreen, Reveal } from '@/lib/evening'
import { MAX_SECONDS, browserRecorder, type Heard, type Recorder, type Recording } from '@/lib/voice'
import { cn } from '@/lib/utils'
import { Btn, Panel, field } from './ui'

const KINDS: CopilotKind[] = ['describe', 'npc', 'consequence', 'next', 'free']

function revealOf(a: CopilotAction): Reveal {
  switch (a.type) {
    case 'reveal_clue':
      return { kind: 'clue', clue: a.clue }
    case 'advance_front':
      return { kind: 'front', front: a.front, delta: 1 }
    case 'enter_scene':
      return { kind: 'scene', node: a.node }
    case 'reveal_npc':
      return { kind: 'npc', npc: a.npc }
  }
}

/**
 * The co-GM (copilot/draft-narration): the GM asks for a description, an
 * NPC's lines, a consequence, what comes next, or anything; the answer
 * is a draft only the GM reads. They edit it and « Montrer » sends their
 * text to the table, or they drop it. A suggestion with a gesture on the
 * campaign's ids is one tap. Each ask is a counted AI call.
 *
 * The GM may also speak instead of typing (copilot/listen-by-voice): one
 * touch starts the microphone, another sends what was said; it is written
 * down and asked with the chosen kind, and the answer is a draft like any
 * other. What was heard stays on this screen; « Changer » puts it in the
 * text field to correct and ask again.
 */
export function CopilotPanel({
  screen,
  onAsk,
  onDictate,
  onShow,
  onDismiss,
  onReveal,
  recorder = browserRecorder,
}: {
  screen: LiveScreen
  onAsk: (kind: CopilotKind, prompt: string, npc?: string) => Promise<void>
  /** Send a recorded dictation; `null` when the server refused it (the page says why). */
  onDictate: (audio: string, kind: CopilotKind, npc?: string) => Promise<Heard | null>
  onShow: (draft: string, narration: string, lines: { speaker: string; text: string }[]) => void
  onDismiss: (draft: string) => void
  onReveal: (r: Reveal) => void
  recorder?: Recorder
}) {
  const { t } = useTranslation()
  const [kind, setKind] = useState<CopilotKind>('describe')
  const [prompt, setPrompt] = useState('')
  const [npc, setNpc] = useState('')
  const [busy, setBusy] = useState(false)
  const live = screen.session?.status === 'live'
  const drafts = screen.drafts.filter((d) => d.status === 'draft')
  const spent = screen.ai.spending
  return (
    <Panel
      title={t('gmLive.copilot.title')}
      actions={
        <span className="text-caption text-mute-soft">
          {t('gmLive.copilot.budget', {
            spent: (spent.spentMicros / 1_000_000).toFixed(2),
            budget: (spent.budgetMicros / 1_000_000).toFixed(2),
          })}
        </span>
      }
    >
      {!screen.ai.configured && <p className="text-caption text-mute">{t('gmLive.copilot.notConfigured')}</p>}
      <form
        className="flex flex-col gap-1.5"
        onSubmit={async (e) => {
          e.preventDefault()
          setBusy(true)
          try {
            await onAsk(kind, prompt.trim(), kind === 'npc' ? npc || undefined : undefined)
            setPrompt('')
          } finally {
            setBusy(false)
          }
        }}
      >
        <div className="flex flex-wrap gap-1">
          {KINDS.map((k) => (
            <Btn key={k} main={k === kind} onClick={() => setKind(k)}>
              {t(`gmLive.copilot.kind.${k}`)}
            </Btn>
          ))}
        </div>
        {kind === 'npc' && (
          <select className={field} value={npc} onChange={(e) => setNpc(e.target.value)} aria-label={t('gmLive.copilot.npc')}>
            <option value="">{t('gmLive.copilot.anyNpc')}</option>
            {(screen.scene?.npcs ?? []).map((n) => (
              <option key={n.id} value={n.id}>
                {n.name}
              </option>
            ))}
          </select>
        )}
        {recorder.supported ? (
          <VoiceButton
            recorder={recorder}
            disabled={!live || busy || !screen.ai.configured}
            onBusy={setBusy}
            onRecorded={(audio) => onDictate(audio, kind, kind === 'npc' ? npc || undefined : undefined)}
            onChange={setPrompt}
          />
        ) : (
          <p className="text-caption text-mute-soft">{t('gmLive.copilot.voice.unsupported')}</p>
        )}
        <textarea
          className={field}
          rows={2}
          value={prompt}
          onChange={(e) => setPrompt(e.target.value)}
          placeholder={t('gmLive.copilot.prompt')}
          aria-label={t('gmLive.copilot.prompt')}
        />
        <Btn type="submit" main disabled={!live || busy || !screen.ai.configured}>
          {busy ? t('gmLive.copilot.thinking') : t('gmLive.copilot.ask')}
        </Btn>
      </form>
      {drafts.map((d) => (
        <DraftCard key={d.id} draft={d} onShow={onShow} onDismiss={onDismiss} onReveal={onReveal} />
      ))}
    </Panel>
  )
}

type VoiceState = { kind: 'idle' } | { kind: 'recording'; since: number } | { kind: 'sending' } | { kind: 'denied' }

/**
 * The microphone, push to talk: a touch to start, a touch to send. It
 * stops by itself at the server's limit. The button is large, for a
 * finger on a tablet.
 */
function VoiceButton({
  recorder,
  disabled,
  onBusy,
  onRecorded,
  onChange,
}: {
  recorder: Recorder
  disabled: boolean
  onBusy: (busy: boolean) => void
  onRecorded: (audio: string) => Promise<Heard | null>
  onChange: (transcript: string) => void
}) {
  const { t } = useTranslation()
  const [state, setState] = useState<VoiceState>({ kind: 'idle' })
  const [heard, setHeard] = useState<string | null>(null)
  const [now, setNow] = useState(() => Date.now())
  const recording = useRef<Recording | null>(null)
  // The latest kind and NPC, even for the automatic stop at the limit.
  const send = useRef(onRecorded)
  useEffect(() => {
    send.current = onRecorded
  })

  // A recording left running when the panel goes away is dropped.
  useEffect(() => () => recording.current?.cancel(), [])

  async function finish() {
    const r = recording.current
    if (!r) return
    recording.current = null
    setState({ kind: 'sending' })
    onBusy(true)
    try {
      const { audio } = await r.stop()
      const result = await send.current(audio)
      setHeard(result?.transcript ?? null)
    } finally {
      onBusy(false)
      setState({ kind: 'idle' })
    }
  }

  useEffect(() => {
    if (state.kind !== 'recording') return
    const tick = setInterval(() => {
      setNow(Date.now())
      if (Date.now() - state.since >= MAX_SECONDS * 1000) void finish()
    }, 250)
    return () => clearInterval(tick)
    // `finish` reads only refs and state setters.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [state])

  async function begin() {
    setHeard(null)
    try {
      recording.current = await recorder.start()
      const since = Date.now()
      setNow(since)
      setState({ kind: 'recording', since })
    } catch {
      setState({ kind: 'denied' })
    }
  }

  const listening = state.kind === 'recording'
  const seconds = listening ? Math.floor((now - state.since) / 1000) : 0
  return (
    <div className="flex flex-col gap-1.5">
      <button
        type="button"
        aria-pressed={listening}
        disabled={(disabled && !listening) || state.kind === 'sending'}
        onClick={() => void (listening ? finish() : begin())}
        className={cn(
          'flex min-h-14 items-center gap-3 rounded-button border px-3 text-left disabled:opacity-40',
          listening ? 'border-ivory bg-ivory text-ink' : 'border-line text-chalk hover:bg-surface',
        )}
      >
        <span
          aria-hidden
          className={cn('size-3.5 shrink-0 rounded-full bg-current', listening && 'motion-safe:animate-pulse')}
        />
        <span className="flex flex-col">
          <span className="text-body font-bold">
            {state.kind === 'sending'
              ? t('gmLive.copilot.voice.sending')
              : listening
                ? t('gmLive.copilot.voice.listening', { seconds })
                : t('gmLive.copilot.voice.speak')}
          </span>
          <span className="text-caption opacity-70">
            {listening ? t('gmLive.copilot.voice.touchToSend') : t('gmLive.copilot.voice.touchToTalk')}
          </span>
        </span>
      </button>
      {state.kind === 'denied' && (
        <p role="alert" className="text-caption text-mute">
          {t('gmLive.copilot.voice.denied')}
        </p>
      )}
      {heard && (
        <div className="flex items-start justify-between gap-2 text-caption">
          <span>
            <span className="text-mute-soft">{t('gmLive.copilot.voice.heard')} </span>« {heard} »
          </span>
          <Btn
            onClick={() => {
              onChange(heard)
              setHeard(null)
            }}
          >
            {t('gmLive.copilot.voice.change')}
          </Btn>
        </div>
      )}
    </div>
  )
}

function DraftCard({
  draft,
  onShow,
  onDismiss,
  onReveal,
}: {
  draft: Draft
  onShow: (draft: string, narration: string, lines: { speaker: string; text: string }[]) => void
  onDismiss: (draft: string) => void
  onReveal: (r: Reveal) => void
}) {
  const { t } = useTranslation()
  const [narration, setNarration] = useState(draft.answer.narration)
  const [lines, setLines] = useState(draft.answer.npcLines.map((l) => ({ speaker: l.speaker, text: l.text })))
  return (
    <article className="flex flex-col gap-1.5 rounded-lg border border-dashed border-line-dashed p-2.5">
      <span className="type-label">{t(`gmLive.copilot.kind.${draft.kind}`)}</span>
      <textarea
        className={field}
        rows={3}
        value={narration}
        onChange={(e) => setNarration(e.target.value)}
        aria-label={t('gmLive.copilot.narration')}
      />
      {lines.map((l, i) => (
        <label key={i} className="flex flex-col gap-0.5 text-caption">
          {l.speaker}
          <textarea
            className={field}
            rows={2}
            value={l.text}
            onChange={(e) => setLines((ls) => ls.map((x, j) => (j === i ? { ...x, text: e.target.value } : x)))}
          />
        </label>
      ))}
      {draft.answer.gmNote && <p className="text-caption text-mute-soft">{draft.answer.gmNote}</p>}
      {draft.answer.suggestions.map((s, i) => (
        <div key={i} className="flex items-center justify-between gap-2 text-caption">
          <span>
            {s.label}
            {s.why && <span className="text-mute-soft"> · {s.why}</span>}
          </span>
          {s.action && <Btn onClick={() => onReveal(revealOf(s.action!))}>{t('gmLive.copilot.apply')}</Btn>}
        </div>
      ))}
      <div className="flex gap-1.5">
        <Btn main onClick={() => onShow(draft.id, narration, lines.filter((l) => l.text.trim()))}>
          {t('gmLive.copilot.show')}
        </Btn>
        <Btn onClick={() => onDismiss(draft.id)}>{t('gmLive.copilot.dismiss')}</Btn>
      </div>
    </article>
  )
}
