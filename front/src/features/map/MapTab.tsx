import { useCallback, useEffect, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { ArcadeCluster } from '@/components/game/ArcadeCluster'
import { Hearts } from '@/components/game/Hearts'
import { ApiError } from '@/lib/api'
import {
  fetchBoard,
  fightCommand,
  pathTo,
  walk,
  type BoardView,
  type Cell,
  type Command,
  type FightView,
} from '@/lib/board'
import { playerBackdropUrl } from '@/lib/maps'
import { fetchPlayerMedia, playerImageUrl, type MediaList } from '@/lib/media'
import { cn } from '@/lib/utils'
import { eventLine } from './events'
import { MapCanvas } from './MapCanvas'
import { useImage } from './useImage'
import { useTileset } from './useTileset'

type State = { kind: 'loading' } | { kind: 'error' } | { kind: 'ready'; board: BoardView | null }

/**
 * The Map tab (player/explore-map, player/fight-turn): the map the GM
 * shows, as far as the fog and the GM let this player see it. Out of a
 * fight, a tap on a highlighted cell walks the character there along
 * the path the server checks again. In a fight: whose turn it is, the
 * order with hearts for the party, and on my turn the hand of cards,
 * the target picked on the map, the arcade buttons and the log.
 */
export function MapTab({ campaignId, refreshKey }: { campaignId: string; refreshKey: number }) {
  const { t } = useTranslation()
  const [state, setState] = useState<State>({ kind: 'loading' })
  const [media, setMedia] = useState<MediaList | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [card, setCard] = useState<string | null>(null)
  const [targets, setTargets] = useState<string[]>([])
  const [busy, setBusy] = useState(false)
  const latest = useRef(0)

  const load = useCallback(async () => {
    const request = ++latest.current
    let next: State
    let list: MediaList | null = null
    try {
      const [board, media] = await Promise.all([fetchBoard(campaignId), fetchPlayerMedia(campaignId)])
      next = { kind: 'ready', board }
      list = media
    } catch {
      next = { kind: 'error' }
    }
    if (request !== latest.current) return
    // A failed refetch keeps what is on screen; the next change retries.
    setState((s) => (next.kind === 'error' && s.kind === 'ready' ? s : next))
    setMedia((m) => list ?? m)
  }, [campaignId])

  useEffect(() => {
    void load()
  }, [load, refreshKey])

  const board = state.kind === 'ready' ? state.board : null
  const { tileset, atlases } = useTileset(board?.map ?? null, media, (id) => playerImageUrl(campaignId, id))
  // The board's map id is not the players'; its image is the one shown.
  const backdrop = useImage(board?.map.backdrop?.image ? playerBackdropUrl(campaignId, board.map.id) : null)

  async function send(call: () => Promise<BoardView | null>) {
    setError(null)
    setBusy(true)
    try {
      setState({ kind: 'ready', board: await call() })
    } catch (err) {
      setError(err instanceof ApiError ? err.code : 'UNEXPECTED')
    } finally {
      setBusy(false)
    }
  }

  if (state.kind === 'loading') return <p role="status">{t('play.loading')}</p>
  if (state.kind === 'error') return <p role="alert">{t('play.error')}</p>
  if (!board) return <p className="text-body text-chalk-soft">{t('map.none')}</p>

  const fight = board.fight?.live ? board.fight : null
  const me = board.tokens.find((tk) => tk.mine)
  const mineId = fight?.order.find((f) => f.mine)?.id
  const myCell: Cell | undefined = (mineId && fight?.order.find((f) => f.id === mineId)?.at) || me?.at
  const canMove = fight ? fight.myTurn : Boolean(me)
  const nameOf = (id: string) =>
    fight?.order.find((f) => f.id === id)?.name ??
    board.fight?.order.find((f) => f.id === id)?.name ??
    board.tokens.find((tk) => tk.id === id)?.name ??
    fight?.cards.find((c) => c.id === id)?.name ??
    id

  function tapCell(cell: Cell) {
    if (fight?.myTurn && card) {
      const who = fight.order.find((f) => f.at && f.at[0] === cell[0] && f.at[1] === cell[1])
      if (who) {
        setTargets((ts) => (ts.includes(who.id) ? ts.filter((x) => x !== who.id) : [...ts, who.id]))
        return
      }
    }
    if (!canMove || !myCell) return
    const path = pathTo(myCell, cell, board!.reachable)
    if (!path || path.length === 0) return
    if (fight) void send(() => fightCommand(campaignId, { kind: 'move', path }))
    else void send(() => walk(campaignId, path))
  }

  function play(cmd: Command) {
    void send(() => fightCommand(campaignId, cmd))
    setCard(null)
    setTargets([])
  }

  return (
    <div className="flex flex-col gap-3">
      <div className="flex items-baseline justify-between">
        <h2 className="type-title text-[20px]">{board.map.name}</h2>
        {board.map.ambience.weather && board.map.ambience.weather !== 'clear' && (
          <span className="type-label">{t(`map.weather.${board.map.ambience.weather}`)}</span>
        )}
      </div>
      {fight && <FightHeader fight={fight} />}
      {error && (
        <p role="alert" className="rounded-button border border-stat-atk px-3 py-2 text-body">
          {t(`fight.errors.${error}`, { defaultValue: t('fight.errors.UNEXPECTED') })}
        </p>
      )}
      <MapCanvas
        scene={{
          map: board.map,
          tileset,
          atlases,
          backdrop,
          tokens: board.tokens,
          reachable: canMove ? board.reachable : [],
          selected: targets[0] ?? null,
          highlight: fight?.order.filter((f) => targets.includes(f.id) && f.at).map((f) => f.at!) ?? [],
        }}
        className="max-h-[55dvh]"
        onCell={tapCell}
      />
      {!fight && me && <p className="text-caption text-mute">{t('map.walkHint')}</p>}
      {me?.ghost && <p className="text-caption text-mute">{t('map.ghost')}</p>}
      {fight?.myTurn && (
        <>
          <div className="flex gap-2 overflow-x-auto pb-1" role="group" aria-label={t('fight.hand')}>
            {fight.cards.map((c) => (
              <button
                key={c.id}
                type="button"
                disabled={c.locked}
                aria-pressed={card === c.id}
                title={c.description}
                className={cn(
                  'flex min-w-28 flex-col items-start gap-0.5 rounded-xl border px-3 py-2 text-left disabled:opacity-40',
                  card === c.id ? 'border-ivory bg-ivory text-ink' : 'border-line bg-surface',
                )}
                onClick={() => {
                  setCard(card === c.id ? null : c.id)
                  setTargets([])
                }}
              >
                <span className="text-body font-bold">{c.name}</span>
                <span className="text-caption">
                  {t('fight.range', { range: c.range })}
                  {c.attackBonus !== null ? ` · ${t('fight.bonus', { value: c.attackBonus })}` : ''}
                </span>
              </button>
            ))}
          </div>
          {card && <p className="text-caption text-mute">{t('fight.pickTarget', { count: targets.length })}</p>}
          <ArcadeCluster
            busy={busy}
            main={{
              label: t('fight.play'),
              icon: 'sword',
              disabled: !card,
              onClick: () => card && play({ kind: 'act', action: card, targets }),
            }}
            left={{ label: t('fight.flee'), icon: 'arrow', onClick: () => play({ kind: 'flee' }) }}
            right={{ label: t('fight.endTurn'), icon: 'hourglass', onClick: () => play({ kind: 'endTurn' }) }}
          />
        </>
      )}
      {board.fight && <FightLog fight={board.fight} name={nameOf} />}
      {board.fight && !board.fight.live && board.fight.loot.length > 0 && (
        <section className="flex flex-col gap-1.5">
          <h3 className="type-label">{t('fight.loot')}</h3>
          {board.fight.loot.map((l, i) => (
            <p key={i} className={cn('rounded-button px-3 py-2 text-body', l.toMe ? 'bg-ivory text-ink' : 'border border-line')}>
              {l.toMe ? t('fight.lootMine', { name: l.name }) : l.name}
            </p>
          ))}
        </section>
      )}
    </div>
  )
}

function FightHeader({ fight }: { fight: FightView }) {
  const { t } = useTranslation()
  const active = fight.order.find((f) => f.id === fight.active)
  return (
    <section className="flex flex-col gap-2">
      <p
        role="status"
        className={cn(
          'rounded-button px-3.5 py-2.5 text-body font-bold',
          fight.myTurn ? 'bg-ivory text-ink shadow-ivory-flat' : 'border border-line',
        )}
      >
        {fight.myTurn
          ? t('fight.yourTurn', { name: active?.name ?? '' })
          : t('fight.theirTurn', { name: active?.name ?? '', round: fight.round })}
      </p>
      <ol className="flex gap-1.5 overflow-x-auto" aria-label={t('fight.order')}>
        {fight.order.map((f) => (
          <li
            key={f.id}
            className={cn(
              'flex min-w-20 flex-col gap-1 rounded-lg border px-2 py-1.5 text-caption',
              f.id === fight.active ? 'border-ivory' : 'border-line',
              f.standing !== 'in_fight' && 'opacity-40',
              !f.party && 'bg-[#3a1512]',
            )}
          >
            <span className="truncate font-bold">{f.name}</span>
            {f.hitPoints !== null && f.maxHitPoints !== null && (
              <Hearts hp={f.hitPoints} max={f.maxHitPoints} count={4} px={2} animated={false} />
            )}
            {f.conditions.length > 0 && <span className="text-mute-soft">{f.conditions.join(', ')}</span>}
          </li>
        ))}
      </ol>
    </section>
  )
}

function FightLog({ fight, name }: { fight: FightView; name: (id: string) => string }) {
  const { t } = useTranslation()
  const lines = fight.events
    .map((e) => eventLine(e, name, t))
    .filter((l): l is string => l !== null)
    .slice(-8)
    .reverse()
  if (lines.length === 0) return null
  return (
    <section className="flex flex-col gap-1">
      <h3 className="type-label">{t('fight.log.title')}</h3>
      <ul className="flex flex-col gap-0.5 text-caption text-chalk-soft">
        {lines.map((l, i) => (
          <li key={i}>{l}</li>
        ))}
      </ul>
    </section>
  )
}
