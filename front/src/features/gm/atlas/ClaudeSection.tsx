import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Btn, field } from '@/features/gm/live/ui'
import type { ClaudeDecision, Proposal, ProposalItem } from '@/lib/prep'
import { cn } from '@/lib/utils'

const MARK = { add: '+', set: '~', remove: '−' } as const

/**
 * platform/propose-edits-by-claude — Claude's batches waiting on the GM,
 * in the atlas's side panel: each with its title, a way to the map it is
 * about, and its items (a story edit, or a batch of map operations). The
 * GM keeps one, discards one with a word Claude reads back, or keeps or
 * discards everything left. « Claude a la main » lets Claude's writes
 * land directly — refused while an evening is open.
 */
export function ClaudeSection({
  batches,
  picked,
  hand,
  busy,
  failed,
  onPick,
  onFocus,
  onDecide,
  onHand,
}: {
  batches: readonly Proposal[]
  /** The batch chosen on the map. */
  picked: string | null
  hand: boolean
  busy: boolean
  failed: string | null
  onPick: (id: string | null) => void
  onFocus: (p: Proposal) => void
  onDecide: (p: Proposal, decision: ClaudeDecision) => void
  onHand: (on: boolean) => void
}) {
  const { t } = useTranslation()
  return (
    <section aria-label={t('atlas.claude.title')} className="flex flex-col gap-2">
      <div className="flex flex-wrap items-center gap-1.5">
        <h3 className="type-label text-mute-soft">{t('atlas.claude.title')}</h3>
        <button
          type="button"
          className="pixel-choice ml-auto py-1 pr-2 pb-1.5 text-caption font-semibold"
          aria-pressed={hand}
          disabled={busy}
          title={t('atlas.claude.handHint')}
          onClick={() => onHand(!hand)}
        >
          {t('atlas.claude.hand')}
        </button>
      </div>
      {failed && (
        <p role="alert" className="text-caption text-stat-hp">
          {t(`atlas.claude.errors.${failed}`, { defaultValue: t('atlas.claude.errors.action', { code: failed }) })}
        </p>
      )}
      {batches.length === 0 ? (
        <p className="text-caption text-mute-soft">{hand ? t('atlas.claude.noneHand') : t('atlas.claude.none')}</p>
      ) : (
        <ul className="flex flex-col gap-2.5">
          {batches.map((p) => (
            <BatchCard
              key={p.id}
              proposal={p}
              picked={picked === p.id}
              busy={busy}
              onPick={() => onPick(picked === p.id ? null : p.id)}
              onFocus={() => onFocus(p)}
              onDecide={(d) => onDecide(p, d)}
            />
          ))}
        </ul>
      )}
    </section>
  )
}

function BatchCard({
  proposal: p,
  picked,
  busy,
  onPick,
  onFocus,
  onDecide,
}: {
  proposal: Proposal
  picked: boolean
  busy: boolean
  onPick: () => void
  onFocus: () => void
  onDecide: (d: ClaudeDecision) => void
}) {
  const { t } = useTranslation()
  const [word, setWord] = useState('')
  const items = p.items ?? []
  const waiting = items.filter((i) => i.status === 'pending')
  return (
    <li
      className={cn('flex flex-col gap-1.5 border border-dashed p-2', picked ? 'border-chalk' : 'border-line')}
      data-proposal={p.id}
    >
      <button type="button" className="text-left text-caption font-semibold text-chalk" aria-pressed={picked} onClick={onPick}>
        {t('atlas.claude.mark', { title: p.prompt || t('atlas.claude.untitled') })}
      </button>
      {p.focus && (
        <Btn disabled={busy} onClick={onFocus}>
          {t('atlas.claude.focus')}
        </Btn>
      )}
      <ul className="flex flex-col gap-1">
        {items.map((i) => (
          <li key={i.key} className="flex flex-wrap items-center gap-1.5 text-caption">
            <span className={cn('flex-1', i.status !== 'pending' && 'text-mute-soft line-through')}>
              <ItemText item={i} />
              {i.stale && <span className="ml-1 text-stat-hp">{t('atlas.claude.stale')}</span>}
            </span>
            {i.status === 'pending' ? (
              <>
                <Btn disabled={busy || i.stale} onClick={() => onDecide({ keep: [i.key] })}>
                  {t('atlas.claude.keep')}
                </Btn>
                <Btn disabled={busy} onClick={() => onDecide({ discard: [i.key], word: word.trim() })}>
                  {t('atlas.claude.discard')}
                </Btn>
              </>
            ) : (
              <span className="text-mute-soft">{t(`atlas.claude.status.${i.status}`)}</span>
            )}
          </li>
        ))}
      </ul>
      {waiting.length > 0 && (
        <>
          <input
            className={field}
            aria-label={t('atlas.claude.word')}
            placeholder={t('atlas.claude.wordPlaceholder')}
            value={word}
            maxLength={500}
            onChange={(e) => setWord(e.target.value)}
          />
          <div className="flex flex-wrap gap-1.5">
            <Btn main disabled={busy} onClick={() => onDecide({ all: 'keep' })}>
              {t('atlas.claude.keepAll')}
            </Btn>
            <Btn disabled={busy} onClick={() => onDecide({ all: 'discard', word: word.trim() })}>
              {t('atlas.claude.discardAll')}
            </Btn>
          </div>
        </>
      )}
    </li>
  )
}

/** One item, as the GM reads it: what the edit changes, or which map the operations touch. */
function ItemText({ item }: { item: ProposalItem }) {
  const { t } = useTranslation()
  if (item.map) {
    return <span>{t('atlas.claude.mapItem', { name: item.map.name || item.map.map, count: item.map.ops })}</span>
  }
  const c = item.change
  if (!c) return <span>{t('atlas.claude.edit')}</span>
  const kind = t(`prep.workshop.kind.${c.kind}`, { defaultValue: c.kind })
  const name = c.name || c.id
  return (
    <span>
      <b className="mr-1 font-mono">{MARK[c.op]}</b>
      {c.op === 'set'
        ? t('atlas.claude.setLine', {
            kind,
            name,
            field: t(`prep.review.field.${c.field ?? ''}`, { defaultValue: c.field ?? '' }),
          })
        : t('prep.workshop.line', { kind, name })}
    </span>
  )
}
