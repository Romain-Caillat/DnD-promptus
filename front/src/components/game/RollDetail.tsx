import type { TFunction } from 'i18next'
import { useTranslation } from 'react-i18next'
import type { ModifierSource, OutcomeBand, RollBreakdown, RollTarget } from '@/lib/rules'
import { cn } from '@/lib/utils'
import { STAT_BG, type Stat } from './stats'

/** Glyphs, not words. */
const PLUS = '+'
const MINUS = '−'
const EQUALS = '='

const GOOD: OutcomeBand[] = ['success', 'critical_success']

/**
 * One roll and its calculation, as the rules engine made it
 * (`RollBreakdown`): the face kept, each modifier with where it comes
 * from, the total, what it was rolled against, which of the four bands
 * it landed in, and why — « 8 + 1 = 9 contre Moyen (10) : il manquait 1 ».
 * Phone first; the die takes the colour of the stat it rolls, white for
 * a plain check (MEMORY.md §2).
 */
export function RollDetail({
  roll,
  bandName,
  sourceName,
  targetName,
  stat,
  className,
}: {
  roll: RollBreakdown
  /** The band's name in the campaign's rules (« Réussite critique »). */
  bandName: (band: OutcomeBand) => string
  /** What a modifier's source is called (« Force », « Estocade »). */
  sourceName: (source: ModifierSource) => string
  /** The difficulty's name, when the target is a named one. */
  targetName?: string
  stat?: Stat
  className?: string
}) {
  const { t } = useTranslation()
  const target = roll.target ? targetLabel(roll.target, targetName, t) : null
  const good = roll.band !== null && GOOD.includes(roll.band)

  return (
    <figure
      className={cn('flex flex-col gap-2 rounded-xl border border-line bg-surface p-3', className)}
      data-band={roll.band ?? 'unknown'}
    >
      <div className="flex flex-wrap items-center gap-1.5 text-[22px] font-bold tabular-nums">
        <span
          className={cn(
            'grid size-10 place-items-center rounded-md shadow-ivory-flat',
            stat ? cn(STAT_BG[stat], 'text-on-stat') : 'bg-ivory text-ink',
          )}
          aria-label={t('roll.face', { die: roll.die, face: roll.natural })}
        >
          {roll.natural}
        </span>
        {roll.modifiers.map((m, i) => (
          <span key={i} className="flex items-baseline gap-1">
            <span className="text-mute">{m.value < 0 ? MINUS : PLUS}</span>
            <span>{Math.abs(m.value)}</span>
            <span className="text-caption font-semibold text-chalk-soft">{sourceName(m.source)}</span>
          </span>
        ))}
        <span className="text-mute">{EQUALS}</span>
        <span className="text-[28px]">{roll.total}</span>
      </div>
      {roll.faces.length > 1 && (
        <p className="text-caption text-mute">
          {t(`roll.${roll.advantage}`, { faces: roll.faces.join(' · '), kept: roll.natural })}
        </p>
      )}
      <div className="flex flex-wrap items-center gap-2">
        {target && <span className="text-body text-chalk-soft">{t('roll.against', { target })}</span>}
        {roll.band && (
          <span
            className={cn(
              'rounded-button px-2.5 py-1 text-body font-bold',
              good ? 'bg-ivory text-ink shadow-ivory-flat' : 'border-[1.5px] border-chalk bg-table text-chalk',
            )}
          >
            {bandName(roll.band)}
          </span>
        )}
      </div>
      <figcaption className="text-caption text-chalk-soft">{why(roll, target, t)}</figcaption>
    </figure>
  )
}

type T = TFunction

function targetLabel(target: RollTarget, name: string | undefined, t: T): string {
  switch (target.against) {
    case 'difficulty':
      return name ? t('roll.difficulty', { name, value: target.value }) : String(target.value)
    case 'armor_class':
      return t('roll.armorClass', { value: target.value })
    case 'opposed':
      return t('roll.opposed', { value: target.value })
  }
}

/** Why the roll landed where it did, in one sentence. */
function why(roll: RollBreakdown, target: string | null, t: T): string {
  switch (roll.band) {
    case 'critical_failure':
      return t('roll.why.critical_failure', { face: roll.natural })
    case 'critical_success':
      return t('roll.why.critical_success', { face: roll.natural })
    case 'failure':
      return t('roll.why.failure', {
        total: roll.total,
        target,
        missing: (roll.target?.value ?? roll.total) - roll.total,
      })
    case 'success':
      return t('roll.why.success', { total: roll.total, target })
    default:
      return t('roll.why.unknown')
  }
}
