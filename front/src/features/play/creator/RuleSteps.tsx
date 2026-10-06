import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { CardButton } from '@/components/game/CardButton'
import { GameCard } from '@/components/game/GameCard'
import { ApiError } from '@/lib/api'
import { writeBackstory, type ClassOption, type Named } from '@/lib/creator'
import type { ActionCardView, Backstory } from '@/lib/play'

/** Glyphs, not words. */
const MINUS = '−'
const PLUS = '+'

/** A choice among the rules' peoples or classes (board « Créer », moments 1 and 5). */
export function OptionGrid({
  options,
  selected,
  onPick,
}: {
  options: Named[]
  selected: string | undefined
  onPick: (id: string) => void
}) {
  return (
    <div className="grid grid-cols-2 gap-2">
      {options.map((o) => (
        <button
          key={o.id}
          type="button"
          className="cr-option"
          aria-pressed={selected === o.id}
          onClick={() => onPick(o.id)}
        >
          <b>{o.name}</b>
          {o.description}
        </button>
      ))}
    </div>
  )
}

/**
 * One action card as the rules deal it, with its attack bonus or heal on
 * the gem; greyed while above the character's `level`.
 */
export function ActionCard({ card, width = 104, level = 1 }: { card: ActionCardView; width?: number; level?: number }) {
  const { t } = useTranslation()
  const gem =
    card.attackBonus !== null
      ? { stat: 'atk' as const, value: signed(card.attackBonus) }
      : card.heal
        ? { stat: 'hp' as const, value: card.heal }
        : undefined
  const text = card.damage ? t('creator.class.damage', { amount: card.damage }) : card.description
  return (
    <GameCard
      kind="action"
      title={card.name}
      typeLabel={card.kind}
      text={text}
      value={gem?.value}
      stat={gem?.stat}
      icon={card.heal ? 'spell' : 'sword'}
      width={width}
      state={card.level > level ? 'greyed' : 'normal'}
    />
  )
}

export function signed(n: number): string {
  return n >= 0 ? `+${n}` : `−${Math.abs(n)}`
}

/** The class step: what each class does, then its starting hand. */
export function ClassStep({
  classes,
  selected,
  onPick,
}: {
  classes: ClassOption[]
  selected: string | undefined
  onPick: (id: string) => void
}) {
  const { t } = useTranslation()
  const chosen = classes.find((c) => c.id === selected)
  const cards = chosen?.stats?.cards ?? []
  const starting = cards.filter((c) => c.level <= 1)
  const later = cards.filter((c) => c.level > 1)
  return (
    <div className="flex flex-col gap-3">
      <OptionGrid options={classes} selected={selected} onPick={onPick} />
      {chosen && (
        <>
          <span className="type-label">{t('creator.class.hand', { name: chosen.name })}</span>
          <div className="-mx-4 flex gap-2 overflow-x-auto px-4 pb-2">
            {starting.map((c) => (
              <ActionCard key={c.id} card={c} />
            ))}
          </div>
          {later.length > 0 && (
            <p className="text-caption text-mute">
              {t('creator.class.later', {
                count: later.length,
                levels: [...new Set(later.map((c) => c.level))].join(', '),
              })}
            </p>
          )}
        </>
      )}
    </div>
  )
}

/**
 * The abilities step (moment 6): the class's spread pre-filled, − and +
 * on each score. Going over the spread is explained, never blocked.
 */
export function AbilitiesStep({
  abilities,
  scores,
  defaults,
  budget,
  className,
  gmName,
  onScore,
  onReset,
}: {
  abilities: Named[]
  scores: Record<string, number>
  /** The class's own spread. */
  defaults: Record<string, number>
  budget: number
  className: string
  gmName: string
  onScore: (id: string, score: number) => void
  onReset: () => void
}) {
  const { t } = useTranslation()
  const spent = abilities.reduce((sum, a) => sum + (scores[a.id] ?? 0), 0)
  const left = budget - spent
  return (
    <div className="flex flex-col gap-2">
      <div className="flex items-baseline justify-between rounded-[10px] bg-well px-3 py-2.5 text-[13px]">
        <span>{t('creator.abilities.spread', { name: className })}</span>
        <b className={left < 0 ? 'text-[18px] text-stat-atk' : 'text-[18px]'}>
          {left >= 0 ? t('creator.abilities.left', { count: left }) : t('creator.abilities.over', { count: -left })}
        </b>
      </div>
      {abilities.map((a) => {
        const score = scores[a.id] ?? 0
        const over = left < 0 && score > (defaults[a.id] ?? 0)
        return (
          <div key={a.id} className="cr-ability" data-over={over || undefined}>
            <b>{a.id}</b>
            <span>{a.name}</span>
            <button
              type="button"
              aria-label={t('creator.abilities.less', { name: a.name })}
              disabled={score <= 1}
              onClick={() => onScore(a.id, score - 1)}
            >
              {MINUS}
            </button>
            <strong aria-label={t('creator.abilities.score', { name: a.name, score })}>{score}</strong>
            <button
              type="button"
              aria-label={t('creator.abilities.more', { name: a.name })}
              disabled={score >= 30}
              onClick={() => onScore(a.id, score + 1)}
            >
              {PLUS}
            </button>
          </div>
        )
      })}
      {left < 0 && (
        <div className="cr-warn" role="note">
          <b>{t('creator.abilities.warnTitle', { count: -left })}</b>
          {t('creator.abilities.warn', { budget, gm: gmName })}
        </div>
      )}
      <button type="button" className="cr-chip self-start" onClick={onReset}>
        {t('creator.abilities.reset')}
      </button>
    </div>
  )
}

const QUESTIONS = ['origin', 'loss', 'quest'] as const

/** Why the co-GM could not write, by the server's code. */
const WRITE_ERRORS = {
  AI_BUDGET_EXCEEDED: 'creator.story.errors.budget',
  AI_NOT_CONFIGURED: 'creator.story.errors.unavailable',
  AI_UNAVAILABLE: 'creator.story.errors.unavailable',
} as const

/**
 * The backstory step (moment 7): three short answers, then a paragraph
 * the co-GM writes from them (`copilot/co-write-backstory`) — nothing
 * they did not write — which the player keeps or edits.
 */
export function StoryStep({
  campaignId,
  backstory,
  onChange,
}: {
  campaignId: string
  backstory: Backstory
  onChange: (backstory: Backstory) => void
}) {
  const { t } = useTranslation()
  const [writing, setWriting] = useState(false)
  const [failed, setFailed] = useState<(typeof WRITE_ERRORS)[keyof typeof WRITE_ERRORS] | 'creator.story.errors.failed' | null>(null)
  const answered = QUESTIONS.some((q) => backstory[q]?.trim())

  async function write() {
    setWriting(true)
    setFailed(null)
    try {
      const text = await writeBackstory(campaignId, {
        origin: backstory.origin ?? '',
        loss: backstory.loss ?? '',
        quest: backstory.quest ?? '',
      })
      onChange({ ...backstory, text })
    } catch (err) {
      const code = err instanceof ApiError ? err.code : ''
      setFailed(code in WRITE_ERRORS ? WRITE_ERRORS[code as keyof typeof WRITE_ERRORS] : 'creator.story.errors.failed')
    } finally {
      setWriting(false)
    }
  }

  return (
    <div className="flex flex-col gap-3">
      {QUESTIONS.map((q) => (
        <label key={q} className="flex flex-col gap-1.5">
          <span className="type-label">{t(`creator.story.${q}`)}</span>
          <input
            className="cr-field"
            value={backstory[q] ?? ''}
            maxLength={300}
            onChange={(e) => onChange({ ...backstory, [q]: e.target.value })}
          />
        </label>
      ))}
      <CardButton
        variant="dark"
        size="small"
        title={writing ? t('creator.story.writing') : backstory.text ? t('creator.story.rewrite') : t('creator.story.write')}
        subtitle={t('creator.story.writeSub')}
        disabled={writing || !answered}
        onClick={() => void write()}
      />
      {failed && (
        <p role="alert" className="text-caption text-stat-atk">
          {t(failed)}
        </p>
      )}
      <label className="flex flex-col gap-1.5">
        <span className="type-label">{t('creator.story.text')}</span>
        <textarea
          className="cr-field"
          rows={4}
          value={backstory.text ?? ''}
          maxLength={4000}
          onChange={(e) => onChange({ ...backstory, text: e.target.value })}
        />
      </label>
      <p className="text-caption text-mute">{t('creator.story.hint')}</p>
    </div>
  )
}
