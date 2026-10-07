import { useTranslation } from 'react-i18next'
import { CardButton } from '@/components/game/CardButton'
import { FacetedDie, facesOf } from '@/components/game/FacetedDie'
import type { FightView } from '@/lib/board'
import { cn } from '@/lib/utils'

type Boxes = NonNullable<FightView['order'][number]['deathSaves']>

const SUCCESS = 'bg-[#4ade80]'
const FAILURE = 'bg-stat-atk'

/** Success and failure boxes, as everyone sees them. */
export function SaveBoxes({ boxes, className }: { boxes: Boxes; className?: string }) {
  const { t } = useTranslation()
  const row = (n: number, of: number, tone: string) =>
    Array.from({ length: of }, (_, i) => (
      <span key={i} className={cn('size-2.5 rounded-sm border border-line', i < n && tone)} />
    ))
  return (
    <span
      className={cn('flex flex-col gap-0.5', className)}
      aria-label={t('fight.death.boxes', { successes: boxes.successes, failures: boxes.failures })}
    >
      <span className="flex gap-0.5">{row(boxes.successes, boxes.ofSuccesses, SUCCESS)}</span>
      <span className="flex gap-0.5">{row(boxes.failures, boxes.ofFailures, FAILURE)}</span>
    </span>
  )
}

/**
 * Down at 0 HP (planche « Mourir »): not dead. On my turn I roll the
 * death save — the server rolls and keeps the count; the GM alone
 * confirms a death.
 */
export function DeathSavePanel({ fight, onRoll, busy }: { fight: FightView; onRoll: () => void; busy: boolean }) {
  const { t } = useTranslation()
  const me = fight.order.find((f) => f.mine)
  if (!me?.deathSaves && !fight.deathSave) return null
  const myId = me?.id
  const last = [...fight.events].reverse().find((e) => e.kind === 'death_save' && e.who === myId)
  const boxes = me?.deathSaves
  return (
    <section className="surface-slab flex flex-col items-center gap-2 p-3.5 text-center">
      <span className="type-title text-[22px]">
        {fight.deathSave ? t('fight.death.yourSave') : boxes?.stable ? t('fight.death.stable') : t('fight.death.down')}
      </span>
      {fight.deathSave && (
        <span className="text-body">{t('fight.death.need', { difficulty: fight.deathSave.difficulty })}</span>
      )}
      {!fight.deathSave && !boxes?.stable && <span className="text-caption text-chalk-soft">{t('fight.death.wait')}</span>}
      {last?.kind === 'death_save' && (
        <div className="flex items-center gap-3">
          <FacetedDie key={`${last.natural}-${last.successes}-${last.failures}`} faces={facesOf(last.die)} value={last.natural} />
          <span className="text-body font-bold">{t('fight.death.rolled', { natural: last.natural })}</span>
        </div>
      )}
      {boxes && <SaveBoxes boxes={boxes} className="items-center" />}
      {fight.deathSave && <CardButton title={t('fight.death.roll')} disabled={busy} onClick={onRoll} />}
    </section>
  )
}
