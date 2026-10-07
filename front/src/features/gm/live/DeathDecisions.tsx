import { useTranslation } from 'react-i18next'
import type { GmBoard, GmCommand } from '@/lib/board'
import { Btn } from './ui'

/**
 * engine/save-against-death: a death the engine proposes waits for the
 * GM — confirm it, or decide another outcome. Players see nothing
 * before.
 */
export function DeathDecisions({ enc, onCommand }: { enc: NonNullable<GmBoard['encounter']>; onCommand: (c: GmCommand) => void }) {
  const { t } = useTranslation()
  const proposed = Object.entries(enc.fight.dying ?? {}).filter(([, d]) => d.proposed)
  if (proposed.length === 0) return null
  return (
    <div className="flex flex-col gap-1.5 rounded-md border border-stat-atk p-2" role="alert">
      {proposed.map(([id]) => {
        const who = enc.fight.scene.combatants[id]?.name ?? id
        return (
          <div key={id} className="flex flex-col gap-1">
            <span className="text-caption">{t('gmLive.fight.deathProposed', { who })}</span>
            <div className="flex flex-wrap gap-1.5">
              <Btn main onClick={() => onCommand({ kind: 'death', who: id, call: 'die' })}>
                {t('gmLive.fight.confirmDeath', { who })}
              </Btn>
              <Btn onClick={() => onCommand({ kind: 'death', who: id, call: 'spare' })}>{t('gmLive.fight.spare')}</Btn>
            </div>
          </div>
        )
      })}
    </div>
  )
}
