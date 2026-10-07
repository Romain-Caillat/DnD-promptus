import type { TFunction } from 'i18next'
import type { FightEvent } from '@/lib/board'

/**
 * One fight event as a line of the fight log, in French; `null` for the
 * bookkeeping events nobody reads (the order, turn ends).
 */
export function eventLine(e: FightEvent, name: (id: string) => string, t: TFunction): string | null {
  switch (e.kind) {
    case 'round_started':
      return t('fight.log.round', { round: e.round })
    case 'turn_started':
      return t('fight.log.turn', { who: name(e.who) })
    case 'moved':
      return t('fight.log.moved', { who: name(e.who), cells: e.path.length })
    case 'acted':
      return e.targets.length
        ? t('fight.log.actedOn', { who: name(e.who), action: e.action, targets: e.targets.map(name).join(', ') })
        : t('fight.log.acted', { who: name(e.who), action: e.action })
    case 'flee_failed':
      return t('fight.log.fleeFailed', { who: name(e.who) })
    case 'fled':
      return t('fight.log.fled', { who: name(e.who) })
    case 'defeated':
      return t('fight.log.defeated', { who: name(e.who) })
    case 'ended':
      return e.end.winner === 'party'
        ? t('fight.log.won')
        : e.end.winner === 'opposition'
          ? t('fight.log.lost')
          : t('fight.log.stopped')
    case 'rules': {
      const r = e.event
      switch (r.event) {
        case 'roll':
          return t('fight.log.roll', { who: name(r.roller), total: r.breakdown.total, natural: r.breakdown.natural })
        case 'missed':
          return t('fight.log.missed', { target: name(r.target) })
        case 'damaged':
          return t('fight.log.damaged', { target: name(r.target), amount: r.hp_before - r.hp_after })
        case 'healed':
          return t('fight.log.healed', { target: name(r.target), amount: r.amount })
        case 'condition_applied':
          return t('fight.log.condition', { target: name(r.target), name: r.name })
        case 'condition_ended':
          return t('fight.log.conditionEnded', { target: name(r.target), name: r.name })
        case 'knocked_out':
          return t('fight.log.knockedOut', { target: name(r.target) })
        case 'item_used':
          return t('fight.log.item', { who: name(r.who), item: r.item })
        case 'house_rule':
          return t('fight.log.houseRule', { name: r.name })
        default:
          return null
      }
    }
    default:
      return null
  }
}
