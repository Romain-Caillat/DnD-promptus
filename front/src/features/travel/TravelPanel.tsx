import { useCallback, useEffect, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Btn, field } from '@/features/gm/live/ui'
import { ApiError } from '@/lib/api'
import type { Cell, MapData } from '@/lib/board'
import type { Tileset } from '@/lib/media'
import { fetchGmTravel, gmTravelCommand, type GmTravel, type GmTravelCommand } from '@/lib/travel'
import { cn } from '@/lib/utils'
import { HexMap, type HexRoute } from './HexMap'

const same = (a: Cell, b: Cell) => a[0] === b[0] && a[1] === b[1]

/**
 * The journey from the GM's side (maps/travel-hex-world, board
 * « Voyager », laptop or tablet): pick a place on the world map, rename
 * the two routes the server proposes, watch the table vote and choose;
 * play the day portion by portion; keep one of the events drawn for the
 * portion, worded as the GM likes, or none; launch a group check; set
 * the night's watches and speak to the one awake; enter the place
 * reached. Refetched whenever the board moves (`refreshKey`).
 */
export function TravelPanel({
  campaignId,
  map,
  tileset,
  refreshKey,
}: {
  campaignId: string
  map: MapData
  tileset: Tileset | null
  refreshKey: unknown
}) {
  const { t } = useTranslation()
  const [data, setData] = useState<GmTravel | null>(null)
  const [error, setError] = useState<string | null>(null)
  const latest = useRef(0)

  const load = useCallback(async () => {
    const request = ++latest.current
    let next: GmTravel | null | undefined
    try {
      next = await fetchGmTravel(campaignId)
    } catch {
      // A failed refetch keeps what is on screen; the next change retries.
      next = undefined
    }
    if (request !== latest.current || next === undefined) return
    setData(next)
  }, [campaignId])

  useEffect(() => {
    void load()
  }, [load, refreshKey])

  async function send(cmd: GmTravelCommand) {
    setError(null)
    try {
      latest.current++
      setData(await gmTravelCommand(campaignId, cmd))
    } catch (err) {
      setError(err instanceof ApiError ? err.code : 'UNEXPECTED')
    }
  }

  const tr = data?.travel
  if (!data || !tr) {
    return (
      <>
        <HexMap map={map} tileset={tileset} party={null} />
        <p className="text-caption text-mute">{t('travel.gm.noParty')}</p>
      </>
    )
  }
  const p = tr.party
  const j = tr.journey
  const underWay = Boolean(j && j.chosen !== null && !j.arrived)
  const nameOfPlayer = (id: string) => data.members.find((m) => m.player === id)?.name ?? '?'
  const routes: HexRoute[] = !j
    ? []
    : j.chosen !== null
      ? [{ hexes: (j.routes[j.chosen]?.route.hexes ?? []).slice(p.way?.step ?? 0), look: 'chosen' }]
      : j.routes.map((r) => ({ hexes: r.route.hexes, look: 'other' as const }))
  const place = j && data.places.find((pl) => same(pl.at, j.destination.at))
  const when = p.clock.night
    ? t('travel.night', { day: p.clock.day })
    : t('travel.day', { day: p.clock.day, portion: data.guide.portions[p.clock.portion] ?? '' })

  return (
    <div className="flex flex-col gap-2.5">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <span className="type-label text-chalk">{when}</span>
        <Supplies
          name={data.guide.supplies}
          value={p.supplies}
          perDay={data.guide.perPersonPerDay * data.members.length}
          onSet={(value) => void send({ kind: 'supplies', value })}
        />
      </div>
      {error && (
        <p role="alert" className="rounded-button border border-stat-atk px-3 py-2 text-caption">
          {t(`travel.errors.${error}`, { defaultValue: t('travel.errors.UNEXPECTED') })}
        </p>
      )}
      <HexMap
        map={map}
        tileset={tileset}
        party={p.at}
        routes={routes}
        destination={j?.destination.at ?? null}
        onHex={underWay ? undefined : (cell) => void send({ kind: 'plan', to: cell })}
        className="max-h-[60vh]"
      />
      {!underWay && <p className="text-caption text-mute">{t('travel.gm.pickHint')}</p>}
      {!underWay && (
        <div className="flex flex-wrap gap-1.5">
          {data.places
            .filter((pl) => !same(pl.at, p.at))
            .map((pl) => (
              <Btn key={`${pl.at[0]},${pl.at[1]}`} onClick={() => void send({ kind: 'plan', to: pl.at })}>
                {t(pl.secret ? 'travel.gm.towardsSecret' : 'travel.gm.towards', { place: pl.name })}
              </Btn>
            ))}
        </div>
      )}
      {j && (
        <div className="flex flex-col gap-2 border-t border-line pt-2">
          <div className="flex items-center justify-between gap-2">
            <span className="type-label text-chalk">{t('travel.towards', { place: j.destination.name || '?' })}</span>
            <Btn onClick={() => void send({ kind: 'abandon' })}>{t('travel.gm.abandon')}</Btn>
          </div>
          {j.chosen === null &&
            j.routes.map((r, i) => (
              <RouteEditor
                key={`${i}-${r.name}-${r.description}`}
                name={r.name}
                description={r.description}
                days={r.route.days}
                portions={r.route.portions}
                voters={Object.entries(j.votes)
                  .filter(([, v]) => v === i)
                  .map(([who]) => nameOfPlayer(who))}
                onSave={(name, description) => void send({ kind: 'route', index: i, name, description })}
                onChoose={() => void send({ kind: 'choose', index: i })}
              />
            ))}
          {j.chosen !== null && (
            <p className="text-caption text-chalk-soft">
              {t('travel.progress', {
                route: j.routes[j.chosen]?.name ?? '',
                done: p.way?.step ?? 0,
                total: p.way?.hexes.length ?? 0,
              })}
            </p>
          )}
          {underWay && (
            <div className="flex flex-wrap gap-1.5">
              <Btn main onClick={() => void send({ kind: 'advance' })}>
                {p.clock.night
                  ? t('travel.gm.dawn', { day: p.clock.day + 1 })
                  : t('travel.gm.advance', { portion: data.guide.portions[p.clock.portion] ?? '' })}
              </Btn>
              {!p.clock.night && (
                <Btn onClick={() => void send({ kind: 'advance', hold: true })}>{t('travel.gm.hold')}</Btn>
              )}
            </div>
          )}
          {j.arrived && (
            <div className="flex flex-col gap-1">
              {place?.map ? (
                <Btn main onClick={() => void send({ kind: 'enter' })}>
                  {t('travel.gm.enter', { place: j.destination.name })}
                </Btn>
              ) : (
                <p className="text-caption text-mute">{t('travel.gm.noPlaceMap')}</p>
              )}
            </div>
          )}
          {tr.proposal && (
            // A fresh pick for each portion's draw: an event picked in the
            // last one is not in this one.
            <Proposal
              key={`${tr.proposal.day}-${tr.proposal.portion}`}
              proposal={tr.proposal}
              onKeep={(event, text) => void send({ kind: 'keep', event, text })}
              onSkip={() => void send({ kind: 'skip' })}
            />
          )}
          {j.chosen !== null && <GroupCheck data={data} onSend={(c) => void send(c)} />}
          {p.clock.night && j.chosen !== null && <Watches data={data} onSend={(c) => void send(c)} />}
          {j.events.length > 0 && (
            <ul className="flex flex-col gap-0.5 text-caption text-chalk-soft" aria-label={t('travel.gm.kept')}>
              {j.events.map((e, i) => (
                <li key={i}>
                  {t('travel.day', { day: e.day, portion: e.portion })} — <b>{e.title}</b>
                </li>
              ))}
            </ul>
          )}
        </div>
      )}
    </div>
  )
}

function Supplies({
  name,
  value,
  perDay,
  onSet,
}: {
  name: string
  value: number
  perDay: number
  onSet: (v: number) => void
}) {
  const { t } = useTranslation()
  return (
    <span className="flex items-center gap-1 text-caption">
      {t('travel.supplies', { name, value, perDay })}
      <Btn aria-label={t('travel.gm.less', { name })} onClick={() => onSet(value - 1)}>
        −
      </Btn>
      <Btn aria-label={t('travel.gm.more', { name })} onClick={() => onSet(value + 1)}>
        +
      </Btn>
    </span>
  )
}

function RouteEditor({
  name,
  description,
  days,
  portions,
  voters,
  onSave,
  onChoose,
}: {
  name: string
  description: string
  days: number
  portions: number
  voters: string[]
  onSave: (name: string, description: string) => void
  onChoose: () => void
}) {
  const { t } = useTranslation()
  const [n, setN] = useState(name)
  const [d, setD] = useState(description)
  const dirty = n !== name || d !== description
  return (
    <div className="flex flex-col gap-1 rounded-md border border-line p-2">
      <div className="flex flex-wrap items-center gap-1.5">
        <input className={cn(field, 'flex-1')} value={n} onChange={(e) => setN(e.target.value)} aria-label={t('travel.gm.routeName')} />
        <span className="text-caption text-mute">
          {t('travel.days', { count: days })} · {t('travel.gm.portions', { count: portions })}
        </span>
      </div>
      <input
        className={field}
        value={d}
        placeholder={t('travel.gm.routeDescription')}
        onChange={(e) => setD(e.target.value)}
        aria-label={t('travel.gm.routeDescription')}
      />
      <div className="flex flex-wrap items-center gap-1.5">
        <span className="flex-1 text-caption text-chalk-soft">
          {voters.length > 0 ? t('travel.vote.voters', { names: voters.join(', ') }) : t('travel.gm.noVote')}
        </span>
        {dirty && <Btn onClick={() => onSave(n, d)}>{t('travel.gm.save')}</Btn>}
        <Btn main onClick={onChoose}>
          {t('travel.gm.choose')}
        </Btn>
      </div>
    </div>
  )
}

function Proposal({
  proposal,
  onKeep,
  onSkip,
}: {
  proposal: NonNullable<NonNullable<GmTravel['travel']>['proposal']>
  onKeep: (event: string, text: string) => void
  onSkip: () => void
}) {
  const { t } = useTranslation()
  const [picked, setPicked] = useState<string | null>(null)
  const [text, setText] = useState('')
  return (
    <div className="flex flex-col gap-1.5 border-t border-line pt-2">
      <span className="type-label">{t('travel.gm.proposal', { terrain: proposal.terrain })}</span>
      {proposal.events.map((e) => (
        <button
          key={e.id}
          type="button"
          aria-pressed={picked === e.id}
          className={cn(
            'flex flex-col gap-0.5 rounded-md border px-2.5 py-2 text-left text-caption',
            picked === e.id ? 'border-ivory bg-ivory text-ink' : 'border-line',
          )}
          onClick={() => {
            setPicked(e.id)
            setText(e.text)
          }}
        >
          <b>{e.title}</b>
          <span>{e.text}</span>
          {e.gm_notes && <span className="opacity-70">→ {e.gm_notes}</span>}
        </button>
      ))}
      {picked && (
        <textarea
          className={cn(field, 'min-h-16')}
          value={text}
          onChange={(e) => setText(e.target.value)}
          aria-label={t('travel.gm.eventText')}
        />
      )}
      <div className="flex gap-1.5">
        <Btn main disabled={!picked} onClick={() => picked && onKeep(picked, text)}>
          {t('travel.gm.keep')}
        </Btn>
        <Btn onClick={onSkip}>{t('travel.gm.skip')}</Btn>
      </div>
    </div>
  )
}

function GroupCheck({ data, onSend }: { data: GmTravel; onSend: (c: GmTravelCommand) => void }) {
  const { t } = useTranslation()
  const c = data.travel?.journey?.check ?? null
  const [ability, setAbility] = useState(data.abilities?.[0]?.id ?? '')
  const [difficulty, setDifficulty] = useState(12)
  const [label, setLabel] = useState(t('travel.gm.checkLabel'))
  const open = c && c.success === null
  return (
    <div className="flex flex-col gap-1.5 border-t border-line pt-2">
      <span className="type-label">{t('travel.gm.groupCheck')}</span>
      {c && (
        <div className="flex flex-col gap-0.5 text-caption">
          <span className="font-bold">
            {c.label} · {c.abilityName} {c.difficulty} · {t('travel.gm.needed', { needed: c.needed, count: c.rolls.length })}
          </span>
          {c.rolls.map((r) => (
            <span key={r.character}>
              {r.name} : {r.roll ? `${r.roll.total} (${t(`evening.band.${r.roll.band ?? 'failure'}`)})` : t('travel.check.waiting')}
            </span>
          ))}
          {c.success !== null && <b>{t(c.success ? 'travel.check.success' : 'travel.check.failure')}</b>}
        </div>
      )}
      {open ? (
        <Btn onClick={() => onSend({ kind: 'closeCheck' })}>{t('travel.gm.closeCheck')}</Btn>
      ) : (
        <div className="flex flex-wrap items-center gap-1.5">
          <input className={cn(field, 'flex-1')} value={label} onChange={(e) => setLabel(e.target.value)} aria-label={t('travel.gm.checkWhat')} />
          <select className={field} value={ability} onChange={(e) => setAbility(e.target.value)} aria-label={t('travel.gm.ability')}>
            {(data.abilities ?? []).map((a) => (
              <option key={a.id} value={a.id}>
                {a.name}
              </option>
            ))}
          </select>
          <input
            className={cn(field, 'w-16')}
            type="number"
            min={1}
            max={40}
            value={difficulty}
            onChange={(e) => setDifficulty(Number(e.target.value))}
            aria-label={t('travel.gm.difficulty')}
          />
          <Btn disabled={!ability || !label.trim()} onClick={() => onSend({ kind: 'check', ability, difficulty, label })}>
            {t('travel.gm.launchCheck')}
          </Btn>
        </div>
      )}
    </div>
  )
}

function Watches({ data, onSend }: { data: GmTravel; onSend: (c: GmTravelCommand) => void }) {
  const { t } = useTranslation()
  const w = data.travel?.journey?.watch ?? null
  const [slots, setSlots] = useState<(string | null)[]>(() =>
    data.guide.watches.map((_, i) => data.members[i % Math.max(1, data.members.length)]?.character ?? null),
  )
  const [message, setMessage] = useState('')
  return (
    <div className="flex flex-col gap-1.5 border-t border-line pt-2">
      <span className="type-label">{t('travel.watch.title')}</span>
      {!w ? (
        <>
          {data.guide.watches.map((name, i) => (
            <label key={name} className="flex items-center justify-between gap-2 text-caption">
              {name}
              <select
                className={field}
                value={slots[i] ?? ''}
                onChange={(e) => setSlots((s) => s.map((x, k) => (k === i ? e.target.value || null : x)))}
              >
                <option value="">{t('travel.gm.nobody')}</option>
                {data.members.map((m) => (
                  <option key={m.character} value={m.character}>
                    {m.name}
                  </option>
                ))}
              </select>
            </label>
          ))}
          <Btn onClick={() => onSend({ kind: 'watch', slots })}>{t('travel.gm.setWatches')}</Btn>
        </>
      ) : (
        <>
          {w.slots.map((s, i) => (
            <div key={s.name} className={cn('flex items-center justify-between gap-2 text-caption', w.current === i && 'font-bold')}>
              <span>
                {s.name} · {s.who ?? t('travel.gm.nobody')}
              </span>
              {s.who && (
                <Btn
                  main={w.current !== i}
                  onClick={() => onSend({ kind: 'watchTurn', slot: i, message })}
                >
                  {t('travel.gm.playWatch')}
                </Btn>
              )}
            </div>
          ))}
          <textarea
            className={cn(field, 'min-h-14')}
            value={message}
            placeholder={t('travel.gm.watchMessage')}
            onChange={(e) => setMessage(e.target.value)}
            aria-label={t('travel.gm.watchMessage')}
          />
          {w.woken && <p className="text-caption font-bold">{t('travel.watch.woken')}</p>}
        </>
      )}
    </div>
  )
}
