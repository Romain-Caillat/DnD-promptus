import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import type { CopilotAction, CopilotKind, Draft, LiveScreen, Reveal } from '@/lib/evening'
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
 */
export function CopilotPanel({
  screen,
  onAsk,
  onShow,
  onDismiss,
  onReveal,
}: {
  screen: LiveScreen
  onAsk: (kind: CopilotKind, prompt: string, npc?: string) => Promise<void>
  onShow: (draft: string, narration: string, lines: { speaker: string; text: string }[]) => void
  onDismiss: (draft: string) => void
  onReveal: (r: Reveal) => void
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
