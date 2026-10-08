import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { battleEventLine, battleNames } from '@/features/map/battleEvents'
import { MapCanvas } from '@/features/map/MapCanvas'
import { FACINGS, SPRITE_FACING, type BattleCommand, type Facing, type ShipStanding } from '@/lib/battle'
import type { Cell, GmBoard, TokenView } from '@/lib/board'
import { cn } from '@/lib/utils'
import { Btn, Panel, field } from './ui'

const same = (a: Cell, b: Cell) => a[0] === b[0] && a[1] === b[1]
const OUT: ShipStanding[] = ['struck', 'fled', 'destroyed']

/**
 * A ship battle from the GM's side (engine/support-vehicle-combat): open
 * the battle of a scene; every ship with all its gauges, a gauge nudged
 * or a ship taken out by hand; the crew at their stations, seated anew,
 * the ship's holder (LUMEN) or an absent player made to act; on an enemy
 * turn the co-GM's proposal, accepted as is, or the GM's own manoeuvre
 * (a tap on the map) and shot; the boarding that hands over to the deck
 * fight; the log.
 */
export function BattlePanel({
  data,
  live,
  onStart,
  onCommand,
}: {
  data: GmBoard
  live: boolean
  onStart: (node: string) => void
  onCommand: (c: BattleCommand) => void
}) {
  const { t } = useTranslation()
  const [moving, setMoving] = useState<string | null>(null)
  const [facing, setFacing] = useState<Facing>('e')
  const [crew, setCrew] = useState('')
  const [action, setAction] = useState('')
  const [target, setTarget] = useState('')
  const [attacker, setAttacker] = useState('')
  const [defender, setDefender] = useState('')
  const scenes = data.battles ?? []
  const b = data.battle
  if (scenes.length === 0 && !b) return null

  if (!b || b.status === 'ended') {
    const last = b?.view
    return (
      <Panel title={t('gmLive.battle.title')}>
        <div className="flex flex-wrap gap-1.5">
          {scenes.map((s) => (
            <Btn key={s.node} disabled={!live} onClick={() => onStart(s.node)}>
              {t('gmLive.battle.start', { title: s.title })}
            </Btn>
          ))}
        </div>
        {last && (
          <p className="text-caption text-mute">
            {t('gmLive.battle.ended', {
              result: last.won ? t('battle.won') : last.reason ? t(`battle.reason.${last.reason}`) : t('battle.over'),
            })}
          </p>
        )}
      </Panel>
    )
  }

  const view = b.view
  const name = battleNames(view, t)
  const ships = b.battle.ships
  const afloat = ships.filter((s) => s.standing === 'afloat')
  const tokens: TokenView[] = afloat.map((s) => ({
    id: s.id,
    name: t('battle.shipToken', { arrow: t(`battle.arrow.${s.facing}`), name: s.name }),
    at: s.at,
    party: s.side === 'party',
    mine: false,
    ghost: false,    look: null,
    facing: SPRITE_FACING[s.facing],
    trail: [],
    moves: 0,
  }))
  const reach = moving ? (b.enemyReach[moving] ?? []) : []
  const enemyTurn = !view.crewTurn && b.status === 'live'
  const options = (crew && b.crewOptions[crew]) || []
  const option = options.find((o) => o.id === action)
  const lines = (b.proposal?.events ?? b.events)
    .map((e) => battleEventLine(e, name, t))
    .filter((l): l is string => l !== null)
    .slice(-10)

  function tap(cell: Cell) {
    if (moving && reach.some((c) => same(c, cell))) {
      onCommand({ kind: 'maneuver', ship: moving, to: cell, facing })
      setMoving(null)
    }
  }

  function actForCrew() {
    if (!option) return
    const aim =
      option.aim === 'ship'
        ? (() => {
            const x = option.targets.find((x) => `${x.ship}|${x.weapon ?? ''}` === target)
            return x ? { target: x.ship, weapon: x.weapon ?? undefined } : {}
          })()
        : option.aim === 'damage'
          ? { damage: Number(target) }
          : {}
    onCommand({ kind: 'crew', crew, command: { kind: 'act', action: option.id, aim } })
    setAction('')
    setTarget('')
  }

  return (
    <Panel
      title={t('gmLive.battle.title')}
      actions={<Btn onClick={() => onCommand({ kind: 'stop' })}>{t('gmLive.battle.stop')}</Btn>}
    >
      <p className="type-label">
        {t('gmLive.battle.round', {
          round: view.round,
          who: view.crewTurn ? t('battle.crew') : (view.active ?? ''),
        })}
      </p>
      {b.status === 'boarding' && <p className="text-body font-bold">{t('gmLive.battle.boarding')}</p>}
      <MapCanvas
        scene={{ map: view.map, tileset: null, tokens, reachable: reach.map((at) => ({ at, cost: 0 })), selected: moving }}
        className="max-h-[50vh]"
        onCell={tap}
      />
      {moving && <p className="text-caption text-mute">{t('gmLive.battle.maneuverHint', { name: name(moving) })}</p>}
      <ul className="flex flex-col gap-1">
        {ships.map((s) => (
          <li
            key={s.id}
            className={cn(
              'flex flex-wrap items-center justify-between gap-1.5 rounded-md px-2 py-1 text-caption',
              s.side === 'opposition' && 'bg-[#3a1512]',
              s.standing !== 'afloat' && 'opacity-40',
            )}
          >
            <span className="font-bold">
              {s.name}
              {s.standing !== 'afloat' && ` · ${t(`battle.standing.${s.standing}`)}`}
            </span>
            <span className="tabular-nums">
              {t('battle.gauge', { name: view.gauges.hull, value: s.hull, max: s.max_hull })}
              {view.gauges.screen && s.max_screen > 0 && ` · ${t('battle.gauge', { name: view.gauges.screen, value: s.screen, max: s.max_screen })}`}
              {s.morale !== null && ` · ${t('battle.gauge', { name: view.gauges.morale, value: s.morale, max: s.max_morale ?? s.morale })}`}
              {s.damages.length > 0 && ` · ${s.damages.map((d) => d.name).join(', ')}`}
            </span>
            {s.standing === 'afloat' && (
              <span className="flex flex-wrap gap-1">
                <Btn onClick={() => onCommand({ kind: 'adjust', ship: s.id, hull: s.hull - 1 })}>
                  {t('gmLive.battle.hull', { name: view.gauges.hull })}
                </Btn>
                <Btn onClick={() => onCommand({ kind: 'adjust', ship: s.id, hull: s.hull + 1 })}>
                  {t('gmLive.battle.hullUp', { name: view.gauges.hull })}
                </Btn>
                {s.morale !== null && (
                  <Btn onClick={() => onCommand({ kind: 'adjust', ship: s.id, morale: s.morale! - 1 })}>
                    {t('gmLive.battle.moraleDown')}
                  </Btn>
                )}
                {s.side === 'opposition' && (
                  <select
                    className={field}
                    value=""
                    aria-label={t('gmLive.battle.strike')}
                    onChange={(e) => e.target.value && onCommand({ kind: 'strike', ship: s.id, standing: e.target.value as ShipStanding })}
                  >
                    <option value="">{t('gmLive.battle.strike')}</option>
                    {OUT.map((o) => (
                      <option key={o} value={o}>
                        {t(`battle.standing.${o}`)}
                      </option>
                    ))}
                  </select>
                )}
              </span>
            )}
          </li>
        ))}
      </ul>
      {enemyTurn && (
        <div className="flex flex-col gap-1.5 border-t border-line pt-2">
          <div className="flex flex-wrap gap-1.5">
            <Btn main={!b.proposal} onClick={() => onCommand({ kind: 'propose' })}>
              {t('gmLive.battle.propose')}
            </Btn>
            {b.proposal && (
              <Btn main onClick={() => onCommand({ kind: 'accept' })}>
                {t('gmLive.battle.accept')}
              </Btn>
            )}
            <Btn onClick={() => onCommand({ kind: 'endTurn' })}>{t('gmLive.battle.endTurn')}</Btn>
          </div>
          <div className="flex flex-wrap items-center gap-1.5">
            {Object.keys(b.enemyReach).map((id) => (
              <Btn key={id} main={moving === id} onClick={() => setMoving(moving === id ? null : id)}>
                {t('gmLive.battle.move', { name: name(id) })}
              </Btn>
            ))}
            {Object.keys(b.enemyReach).length > 0 && (
              <select className={field} value={facing} onChange={(e) => setFacing(e.target.value as Facing)} aria-label={t('battle.pickCell')}>
                {FACINGS.map((f) => (
                  <option key={f} value={f}>
                    {t('battle.bow', { facing: t(`battle.facing.${f}`) })}
                  </option>
                ))}
              </select>
            )}
          </div>
          {(b.enemyFire ?? []).length > 0 && (
            <div className="flex flex-wrap gap-1.5">
              {(b.enemyFire ?? []).map((f) => (
                <Btn
                  key={`${f.ship}-${f.weapon}-${f.target}`}
                  onClick={() => onCommand({ kind: 'fire', ship: f.ship, weapon: f.weapon, target: f.target })}
                >
                  {t('gmLive.battle.fireAt', { ship: name(f.ship), weapon: f.name, target: name(f.target) })}
                </Btn>
              ))}
            </div>
          )}
        </div>
      )}
      {view.crewTurn && (
        <div className="flex justify-end">
          <Btn onClick={() => onCommand({ kind: 'endTurn' })}>{t('gmLive.battle.endTurn')}</Btn>
        </div>
      )}
      {b.proposal && <p className="type-label">{t('gmLive.battle.proposal', { unit: b.proposal.unit })}</p>}
      <ul className="flex flex-col gap-0.5 text-caption text-chalk-soft">
        {lines.map((l, i) => (
          <li key={i}>{l}</li>
        ))}
      </ul>
      <div className="flex flex-col gap-1.5 border-t border-line pt-2">
        <span className="type-label">{t('battle.crew')}</span>
        {view.crew.map((c) => (
          <div key={c.id} className="flex flex-wrap items-center justify-between gap-1.5 text-caption">
            <span className={cn('font-bold', c.done && view.crewTurn && 'opacity-50')}>{c.name}</span>
            <select
              className={field}
              value={c.station ?? ''}
              aria-label={t('gmLive.battle.seat', { name: c.name })}
              onChange={(e) => onCommand({ kind: 'seat', crew: c.id, station: e.target.value || null })}
            >
              <option value="">{t('battle.leaveStation')}</option>
              {view.stations.map((s) => (
                <option key={s.id} value={s.id}>
                  {s.name}
                  {s.down ? ` · ${t('battle.stationDown')}` : ''}
                </option>
              ))}
            </select>
          </div>
        ))}
        {view.crewTurn && (
          <div className="flex flex-wrap items-center gap-1.5">
            <select
              className={field}
              value={crew}
              aria-label={t('gmLive.battle.actFor', { name: '' })}
              onChange={(e) => {
                setCrew(e.target.value)
                setAction('')
                setTarget('')
              }}
            >
              <option value="">{t('gmLive.battle.actFor', { name: '' })}</option>
              {view.crew.map((c) => (
                <option key={c.id} value={c.id}>
                  {c.name}
                </option>
              ))}
            </select>
            {crew && (
              <select className={field} value={action} aria-label={t('gmLive.battle.action')} onChange={(e) => setAction(e.target.value)}>
                <option value="">{t('gmLive.battle.action')}</option>
                {options
                  .filter((o) => o.usable && (o.aim === 'none' || o.aim === 'ship' || o.aim === 'damage'))
                  .map((o) => (
                    <option key={o.id} value={o.id}>
                      {o.name}
                    </option>
                  ))}
              </select>
            )}
            {option?.aim === 'ship' && (
              <select className={field} value={target} aria-label={t('gmLive.battle.target')} onChange={(e) => setTarget(e.target.value)}>
                <option value="">{t('gmLive.battle.target')}</option>
                {option.targets.map((x) => (
                  <option key={`${x.ship}|${x.weapon ?? ''}`} value={`${x.ship}|${x.weapon ?? ''}`}>
                    {name(x.ship)}
                  </option>
                ))}
              </select>
            )}
            {option?.aim === 'damage' && (
              <select className={field} value={target} aria-label={t('gmLive.battle.target')} onChange={(e) => setTarget(e.target.value)}>
                <option value="">{t('gmLive.battle.target')}</option>
                {option.damages.map((d) => (
                  <option key={d.index} value={String(d.index)}>
                    {d.name}
                  </option>
                ))}
              </select>
            )}
            <Btn disabled={!option || (option.aim !== 'none' && !target)} onClick={actForCrew}>
              {t('gmLive.battle.do')}
            </Btn>
            {crew && (
              <Btn onClick={() => onCommand({ kind: 'crew', crew, command: { kind: 'pass' } })}>
                {t('gmLive.battle.pass', { name: name(crew) })}
              </Btn>
            )}
          </div>
        )}
      </div>
      {b.status === 'live' && (
        <div className="flex flex-wrap items-center gap-1.5 border-t border-line pt-2">
          <span className="type-label">{t('gmLive.battle.boardTitle')}</span>
          <select className={field} value={attacker} aria-label={t('gmLive.battle.attacker')} onChange={(e) => setAttacker(e.target.value)}>
            <option value="">{t('gmLive.battle.attacker')}</option>
            {afloat.map((s) => (
              <option key={s.id} value={s.id}>
                {s.name}
              </option>
            ))}
          </select>
          <select className={field} value={defender} aria-label={t('gmLive.battle.defender')} onChange={(e) => setDefender(e.target.value)}>
            <option value="">{t('gmLive.battle.defender')}</option>
            {afloat
              .filter((s) => s.id !== attacker)
              .map((s) => (
                <option key={s.id} value={s.id}>
                  {s.name}
                </option>
              ))}
          </select>
          <Btn disabled={!attacker || !defender} onClick={() => onCommand({ kind: 'board', attacker, defender })}>
            {t('gmLive.battle.board')}
          </Btn>
        </div>
      )}
    </Panel>
  )
}
