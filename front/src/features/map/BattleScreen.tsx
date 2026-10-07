import type { TFunction } from 'i18next'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { ArcadeCluster } from '@/components/game/ArcadeCluster'
import { FACINGS, type ActionView, type Aim, type BattleView, type CrewCommand, type Facing, type ShipView } from '@/lib/battle'
import type { Cell, TokenView } from '@/lib/board'
import { cn } from '@/lib/utils'
import { battleEventLine, battleNames } from './battleEvents'
import { MapCanvas } from './MapCanvas'
import type { Scene } from './render'

const same = (a: Cell, b: Cell) => a[0] === b[0] && a[1] === b[1]

/**
 * A ship battle on a phone (engine/support-vehicle-combat): whose turn
 * it is, every ship with what the crew knows of it, my station and its
 * actions — each aimed at what the server says it can reach (a ship in
 * arc and range, a cell and a bow, a split of the power, a damage) — the
 * crew at their posts and the log. The server decides; this only offers
 * what it would accept.
 */
export function BattleScreen({
  battle,
  busy,
  onCommand,
  tileset = null,
  atlases,
}: {
  battle: BattleView
  busy: boolean
  onCommand: (cmd: CrewCommand) => void
  tileset?: Scene['tileset']
  atlases?: Scene['atlases']
}) {
  const { t } = useTranslation()
  const [picked, setPicked] = useState<string | null>(null)
  const [aim, setAim] = useState<Aim>({})
  const name = battleNames(battle, t)
  const me = battle.me
  const action = me?.options.find((o) => o.id === picked) ?? null
  const myStation = battle.stations.find((s) => s.id === me?.station)
  const afloat = battle.ships.filter((s) => s.standing === 'afloat')
  const tokens: TokenView[] = afloat.map((s) => ({
    id: s.id,
    name: t('battle.shipToken', { arrow: t(`battle.arrow.${s.facing}`), name: s.name }),
    at: s.at,
    party: s.party,
    mine: s.party,
    ghost: false,
  }))

  function pick(o: ActionView) {
    setPicked(picked === o.id ? null : o.id)
    const power = o.aim === 'power' && battle.power ? Object.fromEntries(battle.power.channels.map((c) => [c.id, c.level])) : undefined
    setAim(power ? { power } : {})
  }

  function tapCell(cell: Cell) {
    if (!action) return
    if (action.aim === 'cell' && action.reach.some((c) => same(c, cell))) {
      setAim((a) => ({ ...a, to: cell }))
      return
    }
    const ship = afloat.find((s) => same(s.at, cell))
    const target = ship && action.targets.find((x) => x.ship === ship.id)
    if (target) setAim({ target: target.ship, weapon: target.weapon ?? undefined })
  }

  const ready =
    action !== null &&
    action.usable &&
    (action.aim !== 'ship' || Boolean(aim.target)) &&
    (action.aim !== 'cell' || Boolean(aim.to)) &&
    (action.aim !== 'damage' || aim.damage !== undefined)

  function send(cmd: CrewCommand) {
    onCommand(cmd)
    setPicked(null)
    setAim({})
  }

  const used = Object.values(aim.power ?? {}).reduce((a, b) => a + b, 0)

  return (
    <div className="flex flex-col gap-3">
      <h2 className="type-title text-[20px]">{t('battle.title')}</h2>
      <p
        role="status"
        className={cn(
          'rounded-button px-3.5 py-2.5 text-body font-bold',
          me?.myTurn ? 'bg-ivory text-ink shadow-ivory-flat' : 'border border-line',
        )}
      >
        {statusLine(battle, t, myStation?.name ?? '')}
      </p>
      <ShipStrip ships={battle.ships} gauges={battle.gauges} />
      <MapCanvas
        scene={{
          map: battle.map,
          tileset,
          atlases,
          tokens,
          reachable: action?.aim === 'cell' ? action.reach.map((at) => ({ at, cost: 0 })) : [],
          selected: aim.target ?? null,
          highlight: aim.to ? [aim.to] : [],
        }}
        className="max-h-[45dvh]"
        onCell={tapCell}
      />
      {me && (
        <section className="flex flex-col gap-2" aria-label={t('battle.actions')}>
          <div className="flex items-baseline justify-between gap-2">
            <h3 className="type-label">{myStation ? t('battle.myStation', { name: myStation.name }) : t('battle.noStation')}</h3>
            <span className="text-caption text-mute">{t('battle.left', { actions: me.actions, attacks: me.attacks })}</span>
          </div>
          {myStation?.down && <p className="text-caption text-stat-atk">{t('battle.stationDown')}</p>}
          <div className="flex gap-2 overflow-x-auto pb-1">
            {me.options.map((o) => (
              <button
                key={o.id}
                type="button"
                disabled={!me.myTurn || !o.usable}
                aria-pressed={picked === o.id}
                title={o.description}
                className={cn(
                  'flex min-w-32 flex-col items-start gap-0.5 rounded-xl border px-3 py-2 text-left disabled:opacity-40',
                  picked === o.id ? 'border-ivory bg-ivory text-ink' : 'border-line bg-surface',
                )}
                onClick={() => pick(o)}
              >
                <span className="text-body font-bold">{o.name}</span>
                <span className="text-caption">
                  {t('battle.cost', { cost: o.cost })}
                  {o.check !== null ? ` · ${t('battle.check', { dc: o.check })}` : ''}
                </span>
              </button>
            ))}
          </div>
          {action && <AimPicker action={action} aim={aim} setAim={setAim} battle={battle} name={name} used={used} />}
          {me.myTurn && (
            <ArcadeCluster
              busy={busy}
              main={{
                label: t('battle.act'),
                icon: 'sword',
                disabled: !ready,
                onClick: () => action && send({ kind: 'act', action: action.id, aim }),
              }}
              right={{ label: t('battle.pass'), icon: 'hourglass', onClick: () => send({ kind: 'pass' }) }}
            />
          )}
          {me.myTurn && me.actions >= me.stationCost && (
            <select
              className="rounded-button border border-line bg-table px-2 py-1.5 text-body text-chalk"
              value=""
              aria-label={t('battle.changeStation', { cost: me.stationCost })}
              onChange={(e) => e.target.value && send({ kind: 'station', station: e.target.value === '-' ? null : e.target.value })}
            >
              <option value="">{t('battle.changeStation', { cost: me.stationCost })}</option>
              {battle.stations
                .filter((s) => s.id !== me.station && !s.holder)
                .map((s) => (
                  <option key={s.id} value={s.id}>
                    {s.name}
                  </option>
                ))}
              <option value="-">{t('battle.leaveStation')}</option>
            </select>
          )}
        </section>
      )}
      <section className="flex flex-col gap-1" aria-label={t('battle.crew')}>
        <h3 className="type-label">{t('battle.crew')}</h3>
        <ul className="flex flex-wrap gap-1.5">
          {battle.crew.map((c) => (
            <li
              key={c.id}
              className={cn(
                'rounded-lg border px-2 py-1 text-caption',
                c.mine ? 'border-ivory' : 'border-line',
                c.done && battle.crewTurn && 'opacity-50',
              )}
            >
              {t('battle.atStation', { name: c.name, station: c.station ? name(c.station) : '—' })}
            </li>
          ))}
        </ul>
      </section>
      <BattleLog battle={battle} name={name} />
    </div>
  )
}

function statusLine(b: BattleView, t: TFunction, station: string): string {
  if (!b.live) return b.won ? t('battle.won') : b.reason ? t(`battle.reason.${b.reason}`) : t('battle.over')
  if (b.boarding) return t('battle.boarding')
  if (b.me?.myTurn) return t('battle.yourTurn', { name: station })
  if (b.crewTurn) return b.me ? t('battle.done') : t('battle.crewTurn', { round: b.round })
  return t('battle.enemyTurn', { round: b.round, name: b.active ?? '' })
}

function ShipStrip({ ships, gauges }: { ships: ShipView[]; gauges: BattleView['gauges'] }) {
  const { t } = useTranslation()
  const gauge = (label: string | null, value: number | null, max: number | null) =>
    label && value !== null && max !== null && max > 0 ? t('battle.gauge', { name: label, value, max }) : null
  return (
    <ol className="flex gap-1.5 overflow-x-auto" aria-label={t('battle.ships')}>
      {ships.map((s) => (
        <li
          key={s.id}
          className={cn(
            'flex min-w-28 flex-col gap-0.5 rounded-lg border px-2 py-1.5 text-caption',
            s.party ? 'border-ivory' : 'border-line bg-[#3a1512]',
            s.standing !== 'afloat' && 'opacity-40',
          )}
        >
          <span className="truncate font-bold">{s.name}</span>
          {s.standing !== 'afloat' && <span>{t(`battle.standing.${s.standing}`)}</span>}
          {s.known ? (
            [
              gauge(gauges.hull, s.hull, s.maxHull),
              gauge(gauges.screen, s.screen, s.maxScreen),
              gauge(gauges.morale, s.morale, s.maxMorale),
            ]
              .filter((x): x is string => x !== null)
              .map((x) => (
                <span key={x} className="tabular-nums">
                  {x}
                </span>
              ))
          ) : (
            <span className="text-mute-soft">{t('battle.unknownGauges')}</span>
          )}
          {s.damages.length > 0 && <span className="text-stat-atk">{s.damages.join(', ')}</span>}
        </li>
      ))}
    </ol>
  )
}

function AimPicker({
  action,
  aim,
  setAim,
  battle,
  name,
  used,
}: {
  action: ActionView
  aim: Aim
  setAim: (a: Aim | ((a: Aim) => Aim)) => void
  battle: BattleView
  name: (id: string) => string
  used: number
}) {
  const { t } = useTranslation()
  const chip = (on: boolean) =>
    cn('rounded-button border px-3 py-1.5 text-body', on ? 'border-ivory bg-ivory text-ink' : 'border-line')
  switch (action.aim) {
    case 'ship':
      return action.targets.length === 0 ? (
        <p className="text-caption text-mute">{t('battle.noTarget')}</p>
      ) : (
        <div className="flex flex-wrap gap-1.5" role="group" aria-label={t('battle.pickShip')}>
          {action.targets.map((x) => (
            <button
              key={`${x.ship}-${x.weapon ?? ''}`}
              type="button"
              aria-pressed={aim.target === x.ship && (aim.weapon ?? null) === x.weapon}
              className={chip(aim.target === x.ship && (aim.weapon ?? null) === x.weapon)}
              onClick={() => setAim({ target: x.ship, weapon: x.weapon ?? undefined })}
            >
              {name(x.ship)}
            </button>
          ))}
        </div>
      )
    case 'cell':
      return (
        <div className="flex flex-col gap-1.5">
          <p className="text-caption text-mute">{t('battle.pickCell')}</p>
          <div className="flex flex-wrap gap-1.5">
            {FACINGS.map((f: Facing) => (
              <button
                key={f}
                type="button"
                aria-pressed={aim.facing === f}
                className={chip(aim.facing === f)}
                onClick={() => setAim((a) => ({ ...a, facing: f }))}
              >
                {t('battle.bow', { facing: t(`battle.facing.${f}`) })}
              </button>
            ))}
          </div>
        </div>
      )
    case 'damage':
      return action.damages.length === 0 ? (
        <p className="text-caption text-mute">{t('battle.noDamage')}</p>
      ) : (
        <div className="flex flex-wrap gap-1.5" role="group" aria-label={t('battle.pickDamage')}>
          {action.damages.map((d) => (
            <button
              key={d.index}
              type="button"
              aria-pressed={aim.damage === d.index}
              className={chip(aim.damage === d.index)}
              onClick={() => setAim({ damage: d.index })}
            >
              {d.name}
            </button>
          ))}
        </div>
      )
    case 'power': {
      const p = battle.power
      if (!p) return null
      return (
        <div className="flex flex-col gap-1.5">
          <p className="text-caption text-mute">{t('battle.powerLeft', { used, points: p.points })}</p>
          {p.channels.map((c) => {
            const level = aim.power?.[c.id] ?? c.level
            const set = (n: number) => setAim((a) => ({ ...a, power: { ...(a.power ?? {}), [c.id]: n } }))
            return (
              <div key={c.id} className="flex items-center justify-between gap-2 text-caption">
                <span className="font-bold">{c.name}</span>
                <span className="flex items-center gap-1.5">
                  <button
                    type="button"
                    className={chip(false)}
                    disabled={level <= 0}
                    onClick={() => set(level - 1)}
                    aria-label={t('battle.lower', { name: c.name })}
                  >
                    {t('battle.minus')}
                  </button>
                  <span className="tabular-nums">{level}</span>
                  <button
                    type="button"
                    className={chip(false)}
                    disabled={level >= c.max || used >= p.points}
                    onClick={() => set(level + 1)}
                    aria-label={t('battle.raise', { name: c.name })}
                  >
                    {t('battle.plus')}
                  </button>
                </span>
              </div>
            )
          })}
        </div>
      )
    }
    default:
      return null
  }
}

function BattleLog({ battle, name }: { battle: BattleView; name: (id: string) => string }) {
  const { t } = useTranslation()
  const lines = battle.events
    .map((e) => battleEventLine(e, name, t))
    .filter((l): l is string => l !== null)
    .slice(-8)
    .reverse()
  if (lines.length === 0) return null
  return (
    <section className="flex flex-col gap-1">
      <h3 className="type-label">{t('battle.log.title')}</h3>
      <ul className="flex flex-col gap-0.5 text-caption text-chalk-soft">
        {lines.map((l, i) => (
          <li key={i}>{l}</li>
        ))}
      </ul>
    </section>
  )
}
