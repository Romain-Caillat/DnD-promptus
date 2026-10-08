import type { TFunction } from 'i18next'
import type { BattleEvent } from '@/lib/battle'

/**
 * One ship-battle event as a line of the log, in French; `null` for the
 * bookkeeping nobody reads (initiative, turn ends, a GM's adjustment).
 * `name` turns a ship, crew, unit or station id into what the table
 * calls it.
 */
export function battleEventLine(e: BattleEvent, name: (id: string) => string, t: TFunction): string | null {
  switch (e.kind) {
    case 'round_started':
      return t('battle.log.round', { round: e.round })
    case 'turn_started':
      return t('battle.log.turn', { who: name(e.unit) })
    case 'seated':
      return e.station
        ? t('battle.log.seated', { who: name(e.crew), station: name(e.station) })
        : t('battle.log.unseated', { who: name(e.crew) })
    case 'maneuvered':
      return t('battle.log.maneuvered', { ship: name(e.ship), facing: t(`battle.facing.${e.facing}`) })
    case 'fired':
      return e.hit
        ? t('battle.log.hit', { ship: name(e.ship), target: name(e.target), damage: e.damage })
        : t('battle.log.missed', { ship: name(e.ship), target: name(e.target) })
    case 'evading':
      return t('battle.log.evading', { ship: name(e.ship), armor: e.armor })
    case 'braced':
      return t('battle.log.braced', { ship: name(e.ship) })
    case 'recharged':
      return t('battle.log.recharged', { ship: name(e.ship), screen: e.screen })
    case 'repaired':
      return t('battle.log.repaired', { who: name(e.by), amount: e.amount })
    case 'rerouted':
      return t('battle.log.rerouted', { ship: name(e.ship) })
    case 'locked':
      return t('battle.log.locked', { who: name(e.by), target: name(e.target) })
    case 'scanned':
      return t('battle.log.scanned', { who: name(e.by), target: name(e.target) })
    case 'jammed':
      return t('battle.log.jammed', { who: name(e.by), target: name(e.target) })
    case 'morale_roll':
      return t('battle.log.morale', { who: name(e.by), target: name(e.target), loss: e.loss })
    case 'rallied':
      return t('battle.log.rallied', { who: name(e.by), bonus: e.bonus })
    case 'check':
      return t('battle.log.check', { who: name(e.by), natural: e.roll.natural, total: e.roll.total })
    case 'for_the_gm':
      return t('battle.log.forTheGm', { who: name(e.by) })
    case 'damage_rolled':
      return t('battle.log.damage', { ship: name(e.ship), name: e.name })
    case 'damage_fixed':
      return t('battle.log.fixed', { who: name(e.by) })
    case 'burned':
      return t('battle.log.burned', { ship: name(e.ship), amount: e.amount })
    case 'ship_out':
      return t('battle.log.out', { ship: name(e.ship), standing: t(`battle.standing.${e.standing}`) })
    case 'current_changed':
      return t('battle.log.current')
    case 'boarding_started':
      return t('battle.log.boardingStarted', { attacker: name(e.attacker), defender: name(e.defender) })
    case 'boarding_ended':
      return e.attacker_won
        ? t('battle.log.boardingWon', { defender: name(e.defender) })
        : t('battle.log.boardingLost', { attacker: name(e.attacker) })
    case 'ended':
      return e.end.winner === 'party'
        ? t('battle.log.won')
        : e.end.winner === 'opposition'
          ? t('battle.log.lost')
          : t('battle.log.ended')
    default:
      return null
  }
}

/** Ids of a battle to names: ships, crew, stations; a squad by its word. */
export function battleNames(
  view: { ships: { id: string; name: string }[]; crew: { id: string; name: string }[]; stations: { id: string; name: string }[] },
  t: TFunction,
): (id: string) => string {
  const all = new Map<string, string>()
  for (const x of [...view.stations, ...view.crew, ...view.ships]) all.set(x.id, x.name)
  return (id) => all.get(id) ?? (id.startsWith('squad:') ? t('battle.squad') : id)
}
