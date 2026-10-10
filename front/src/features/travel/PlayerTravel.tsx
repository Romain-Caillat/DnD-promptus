import type { TFunction } from 'i18next'
import { useCallback, useEffect, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { CardButton } from '@/components/game/CardButton'
import { FacetedDie, facesOf } from '@/components/game/FacetedDie'
import { RollDetail } from '@/components/game/RollDetail'
import { ApiError } from '@/lib/api'
import type { BoardView } from '@/lib/board'
import type { Tileset } from '@/lib/media'
import { fetchTravel, travelCommand, type PlayerCommand, type TravelView } from '@/lib/travel'
import { cn } from '@/lib/utils'
import { HexMap, type HexRoute } from './HexMap'

/** The journey as this player may see it, refetched when `refreshKey` moves. */
function useTravel(campaignId: string, refreshKey: number) {
  const [travel, setTravel] = useState<TravelView | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [busy, setBusy] = useState(false)
  const latest = useRef(0)

  const load = useCallback(async () => {
    const request = ++latest.current
    let next: TravelView | null | undefined
    try {
      next = await fetchTravel(campaignId)
    } catch {
      // A failed refetch keeps what is on screen; the next change retries.
      next = undefined
    }
    if (request !== latest.current || next === undefined) return
    setTravel(next)
  }, [campaignId])

  useEffect(() => {
    void load()
  }, [load, refreshKey])

  async function send(cmd: PlayerCommand) {
    setError(null)
    setBusy(true)
    try {
      latest.current++
      setTravel(await travelCommand(campaignId, cmd))
    } catch (err) {
      setError(err instanceof ApiError ? err.code : 'UNEXPECTED')
    } finally {
      setBusy(false)
    }
  }

  return { travel, error, busy, send }
}

function TravelError({ code }: { code: string | null }) {
  const { t } = useTranslation()
  if (!code) return null
  return (
    <p role="alert" className="rounded-button border border-stat-atk px-3 py-2 text-body">
      {t(`travel.errors.${code}`, { defaultValue: t('travel.errors.UNEXPECTED') })}
    </p>
  )
}

/** « Jour 2 · Après-midi » or « Nuit du jour 2 ». */
function when(t: TFunction, v: TravelView): string {
  return v.night ? t('travel.night', { day: v.day }) : t('travel.day', { day: v.day, portion: v.portion ?? '' })
}

/**
 * The Map tab on a world map (maps/travel-hex-world, board « Voyager »,
 * phone first): the hexes as far as the fog lets the table see, the party
 * as one pin, the day and the supplies; while the GM proposes routes, a
 * card per route to vote with — then the road taken and how far along
 * it the party is. Nobody drags the party: the GM plays the days.
 */
export function WorldMapTab({
  campaignId,
  board,
  tileset,
  refreshKey,
}: {
  campaignId: string
  board: BoardView
  tileset: Tileset | null
  refreshKey: number
}) {
  const { t } = useTranslation()
  const { travel, error, busy, send } = useTravel(campaignId, refreshKey)
  const party = board.tokens.find((tk) => tk.party)?.at ?? null
  const j = travel?.journey ?? null
  const voting = j && j.chosen === null && !j.arrived
  const routes: HexRoute[] = !j
    ? []
    : j.chosen !== null
      ? [{ hexes: (j.routes[j.chosen]?.hexes ?? []).slice(j.progress?.[0] ?? 0), look: 'chosen' }]
      : j.routes.map((r) => ({ hexes: r.hexes, look: r.mine ? 'mine' : 'other' }))

  return (
    <div className="flex flex-col gap-3">
      <div className="flex items-baseline justify-between gap-2">
        <h2 className="type-title text-[16px]">{board.map.name}</h2>
        {travel && <span className="type-label">{when(t, travel)}</span>}
      </div>
      {travel && (
        <p className="text-caption text-chalk-soft">
          {t('travel.supplies', {
            name: travel.supplies.name,
            value: travel.supplies.value,
            perDay: travel.supplies.perDay,
          })}
        </p>
      )}
      <TravelError code={error} />
      <HexMap
        map={board.map}
        tileset={tileset}
        party={party}
        routes={routes}
        destination={j?.destinationAt ?? null}
        className="max-h-[55dvh]"
      />
      {j && (
        <p className="text-body font-bold">
          {j.destination ? t('travel.towards', { place: j.destination }) : t('travel.towardsUnknown')}
        </p>
      )}
      {voting && (
        <section className="flex flex-col gap-2" aria-label={t('travel.vote.title')}>
          <h3 className="type-label">{t('travel.vote.title')}</h3>
          {j.routes.map((r, i) => (
            <div key={i} className="flex flex-col gap-1">
              <CardButton
                title={r.name}
                subtitle={[t('travel.days', { count: r.days }), r.description || r.terrains.map(([n]) => n).join(', ')]
                  .filter(Boolean)
                  .join(' · ')}
                variant={r.mine ? 'ivory' : 'dark'}
                pressed={r.mine}
                disabled={busy}
                aria-pressed={r.mine}
                onClick={() => void send({ kind: 'vote', route: i })}
              />
              {r.voters.length > 0 && (
                <p className="text-caption text-mute">{t('travel.vote.voters', { names: r.voters.join(', ') })}</p>
              )}
            </div>
          ))}
          <p className="text-caption text-mute">{t('travel.vote.hint')}</p>
        </section>
      )}
      {j && j.chosen !== null && !j.arrived && j.progress && (
        <p className="text-caption text-chalk-soft">
          {t('travel.progress', {
            route: j.routes[j.chosen]?.name ?? '',
            done: j.progress[0],
            total: j.progress[1],
          })}
        </p>
      )}
      {j?.arrived && <p className="rounded-button bg-ivory px-3 py-2 text-body text-ink">{t('travel.arrived')}</p>}
    </div>
  )
}

/**
 * The journey's moments on the Game tab: the event the GM kept, read
 * aloud; the group check, rolled by the server when the player taps;
 * the night's watch — the GM's words on the watcher's phone only, and
 * the choice to wake the others.
 */
export function TravelMoments({ campaignId, refreshKey }: { campaignId: string; refreshKey: number }) {
  const { t } = useTranslation()
  const { travel, error, busy, send } = useTravel(campaignId, refreshKey)
  const j = travel?.journey
  if (!travel || !j) return null
  const last = j.events.at(-1)
  const c = j.check
  const w = j.watch
  const watcher = w && w.current !== null ? w.slots[w.current] : null

  return (
    <section className="flex flex-col gap-3" aria-label={t('travel.title')}>
      <span className="type-label">
        {t('travel.title')} · {when(t, travel)}
      </span>
      <TravelError code={error} />
      {last && (
        <article className="surface-slab flex flex-col gap-1 p-3.5">
          <span className="type-label">{t('travel.onTheRoad')}</span>
          <h3 className="type-title text-[16px]">{last.title}</h3>
          <p className="type-narration text-[18px] leading-snug text-chalk-soft">{last.text}</p>
        </article>
      )}
      {c && (
        <article className="surface-slab flex flex-col gap-2 p-3.5">
          <span className="type-label">{t('travel.check.title', { label: c.label })}</span>
          <p className="text-body">
            {t('travel.check.need', { ability: c.abilityName, difficulty: c.difficulty, needed: c.needed })}
          </p>
          {c.canRoll && (
            <CardButton
              title={t('travel.check.roll')}
              subtitle={c.abilityName}
              sheen
              disabled={busy}
              onClick={() => void send({ kind: 'roll' })}
            />
          )}
          {c.myRoll && (
            <div className="flex items-center gap-3">
              <FacetedDie faces={facesOf(c.myRoll.die)} value={c.myRoll.natural} />
              <RollDetail
                roll={c.myRoll}
                bandName={(b) => t(`evening.band.${b}`)}
                sourceName={() => c.abilityName}
              />
            </div>
          )}
          <ul className="flex flex-wrap gap-1.5" aria-label={t('travel.check.rolls')}>
            {c.rolls.map((r, i) => (
              <li
                key={i}
                className={cn(
                  'rounded-full border px-2.5 py-1 text-caption',
                  r.success === true ? 'border-ivory' : 'border-line',
                  r.mine && 'font-bold',
                )}
              >
                {r.name} · {r.total ?? t('travel.check.waiting')}
              </li>
            ))}
          </ul>
          {c.success !== null && (
            <p
              role="status"
              className={cn(
                'rounded-button px-3 py-2 text-body font-bold',
                c.success ? 'bg-ivory text-ink' : 'border border-line',
              )}
            >
              {t(c.success ? 'travel.check.success' : 'travel.check.failure')}
            </p>
          )}
        </article>
      )}
      {w && (
        <article className={cn('surface-slab flex flex-col gap-2 p-3.5', w.mine && 'border border-stat-init')}>
          <span className="type-label">{t(w.mine ? 'travel.watch.yours' : 'travel.watch.title')}</span>
          {w.mine && w.message && <p className="type-narration text-[18px] leading-snug">{w.message}</p>}
          {!w.mine && watcher && <p className="text-body">{t('travel.watch.awake', { name: watcher.who ?? watcher.name })}</p>}
          {w.mine && (
            <CardButton
              title={t('travel.watch.wake')}
              subtitle={t('travel.watch.wakeHint')}
              disabled={busy || w.woken}
              onClick={() => void send({ kind: 'wake' })}
            />
          )}
          {w.woken && <p className="text-caption text-mute">{t('travel.watch.woken')}</p>}
        </article>
      )}
    </section>
  )
}
