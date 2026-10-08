import { useCallback, useEffect, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { ArcadeCluster } from '@/components/game/ArcadeCluster'
import { CardButton } from '@/components/game/CardButton'
import { Hearts } from '@/components/game/Hearts'
import { Kbd } from '@/components/game/Kbd'
import { ApiError } from '@/lib/api'
import {
  fetchBoard,
  fightCommand,
  pathTo,
  walk,
  type BoardView,
  type Cell,
  type Command,
  type Dying,
  type FightView,
} from '@/lib/board'
import { crewCommand, fetchBattle, type BattleView, type CrewCommand } from '@/lib/battle'
import { playerBackdropUrl } from '@/lib/maps'
import { fetchPlayerMedia, playerImageUrl, type MediaList } from '@/lib/media'
import { cardIndex, cardKey, useShortcuts } from '@/lib/useShortcuts'
import { cn } from '@/lib/utils'
import { BattleScreen } from './BattleScreen'
import { WorldMapTab } from '@/features/travel/PlayerTravel'
import { eventLine } from './events'
import { MapCanvas } from './MapCanvas'
import { useImage } from './useImage'
import { useTileset } from './useTileset'

type State =
  | { kind: 'loading' }
  | { kind: 'error' }
  | { kind: 'ready'; board: BoardView | null; battle: BattleView | null }

/**
 * The Map tab (player/explore-map, player/fight-turn): the map the GM
 * shows, as far as the fog and the GM let this player see it. Out of a
 * fight, a tap on a highlighted cell walks the character there along
 * the path the server checks again. In a fight: whose turn it is, the
 * order with hearts for the party, and on my turn the hand of cards,
 * the target picked on the map, the arcade buttons and the log. In a
 * ship battle (engine/support-vehicle-combat), the battle at my station
 * instead; during a boarding, the fight on the deck. On a computer
 * (player/play-on-desktop) a digit picks a card, Enter plays it, Escape
 * puts it back; `onTurn` tells the page when the keys are the fight's.
 */
export function MapTab({
  campaignId,
  refreshKey,
  keyboard = false,
  onTurn,
}: {
  campaignId: string
  refreshKey: number
  /** Shortcuts on and their hints shown, while it is my turn. */
  keyboard?: boolean
  /** Whether it is my turn in a live fight, each time it changes. */
  onTurn?: (mine: boolean) => void
}) {
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
      const [board, battle, media] = await Promise.all([
        fetchBoard(campaignId),
        fetchBattle(campaignId),
        fetchPlayerMedia(campaignId),
      ])
      next = { kind: 'ready', board, battle }
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
  const battle = state.kind === 'ready' && state.battle?.live ? state.battle : null
  const myTurn = Boolean(board?.fight?.live && board.fight.myTurn)
  useEffect(() => {
    onTurn?.(myTurn)
  }, [myTurn, onTurn])
  const playable = myTurn ? (board?.fight?.cards ?? []) : []
  useShortcuts(keyboard && myTurn, (key) => {
    if (key === 'Escape' && card) {
      setCard(null)
      setTargets([])
      return true
    }
    if (key === 'Enter' && card && !busy) {
      play({ kind: 'act', action: card, targets })
      return true
    }
    const index = cardIndex(key)
    const picked = index === null ? undefined : playable[index]
    if (!picked || picked.locked) return false
    setCard(picked.id)
    setTargets([])
    return true
  })
  const { tileset, atlases } = useTileset(board?.map ?? null, media, (id) => playerImageUrl(campaignId, id))
  const sea = useTileset(battle?.map ?? null, media, (id) => playerImageUrl(campaignId, id))
  // The board's map id is not the players'; its image is the one shown.
  const backdrop = useImage(board?.map.backdrop?.image ? playerBackdropUrl(campaignId, board.map.id) : null)

  async function send(call: () => Promise<BoardView | null>) {
    setError(null)
    setBusy(true)
    try {
      const next = await call()
      setState((s) => ({ kind: 'ready', board: next, battle: s.kind === 'ready' ? s.battle : null }))
    } catch (err) {
      setError(err instanceof ApiError ? err.code : 'UNEXPECTED')
    } finally {
      setBusy(false)
    }
  }

  async function crew(cmd: CrewCommand) {
    setError(null)
    setBusy(true)
    try {
      const next = await crewCommand(campaignId, cmd)
      setState((s) => (s.kind === 'ready' ? { ...s, battle: next } : s))
    } catch (err) {
      setError(err instanceof ApiError ? err.code : 'UNEXPECTED')
    } finally {
      setBusy(false)
    }
  }

  if (state.kind === 'loading') return <p role="status">{t('play.loading')}</p>
  if (state.kind === 'error') return <p role="alert">{t('play.error')}</p>
  if (battle && !battle.boarding) {
    return (
      <div className="flex flex-col gap-3">
        {error && (
          <p role="alert" className="rounded-button border border-stat-atk px-3 py-2 text-body">
            {t(`battle.errors.${error}`, { defaultValue: t('battle.errors.UNEXPECTED') })}
          </p>
        )}
        <BattleScreen battle={battle} busy={busy} onCommand={(c) => void crew(c)} tileset={sea.tileset} atlases={sea.atlases} />
      </div>
    )
  }
  if (!board) return <p className="text-body text-chalk-soft">{t('map.none')}</p>
  if (board.map.scale === 'world') {
    return <WorldMapTab campaignId={campaignId} board={board} tileset={tileset} refreshKey={refreshKey} />
  }

  const fight = board.fight?.live ? board.fight : null
  const me = board.tokens.find((tk) => tk.mine)
  const mineId = fight?.order.find((f) => f.mine)?.id
  const myCell: Cell | undefined = (mineId && fight?.order.find((f) => f.id === mineId)?.at) || me?.at
  const myFighter = fight?.order.find((f) => f.mine)
  const dying = myFighter?.dying ?? null
  const canMove = fight ? fight.myTurn && !fight.deathSave : Boolean(me)
  // Allies dying and not yet stable, whom I may try to stabilise on my turn.
  const toStabilize =
    fight?.myTurn && !fight.deathSave && fight.stabilize
      ? fight.order.filter((f) => !f.mine && f.party && f.dying && !f.dying.stable && f.standing === 'in_fight')
      : []
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
        <h2 className="type-title text-[16px]">{board.map.name}</h2>
        {board.map.ambience.weather && board.map.ambience.weather !== 'clear' && (
          <span className="type-label">{t(`map.weather.${board.map.ambience.weather}`)}</span>
        )}
      </div>
      {battle?.boarding && (
        <p role="status" className="rounded-button border border-ivory px-3 py-2 text-body font-bold">
          {t('battle.boarding')}
        </p>
      )}
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
        fightEvents={board.fight?.events}
        className="max-h-[55dvh]"
        onCell={tapCell}
      />
      {!fight && me && <p className="text-caption text-mute">{t('map.walkHint')}</p>}
      {me?.ghost && <p className="text-caption text-mute">{t('map.ghost')}</p>}
      {fight && dying && <DeathSaves fight={fight} dying={dying} busy={busy} onRoll={() => play({ kind: 'deathSave' })} />}
      {toStabilize.length > 0 && fight?.stabilize && (
        <div className="flex flex-col gap-2">
          {toStabilize.map((f) => (
            <CardButton
              key={f.id}
              variant="dark"
              title={t('fight.death.stabilize', { name: f.name })}
              subtitle={t('fight.death.stabilizeSub', fight.stabilize!)}
              disabled={busy}
              onClick={() => play({ kind: 'stabilize', target: f.id })}
            />
          ))}
        </div>
      )}
      {fight?.myTurn && !fight.deathSave && (
        <>
          <div className="flex gap-2 overflow-x-auto pb-1" role="group" aria-label={t('fight.hand')}>
            {fight.cards.map((c, i) => (
              <button
                key={c.id}
                type="button"
                disabled={c.locked}
                aria-pressed={card === c.id}
                title={c.description}
                className={cn(
                  'pixel-choice flex min-w-28 flex-none flex-col items-start gap-0.5 py-2 pr-3 pb-2.5 text-left',
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
                {keyboard && cardKey(i) && <Kbd>{cardKey(i)!}</Kbd>}
              </button>
            ))}
          </div>
          {card && <p className="text-caption text-mute">{t('fight.pickTarget', { count: targets.length })}</p>}
          {keyboard && <p className="text-caption text-mute">{t('fight.keys')}</p>}
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

/** Three circles of each kind, filled as the server counts them. */
function SaveCircles({ dying }: { dying: Dying }) {
  const { t } = useTranslation()
  const { successes, failures, successesNeeded, failuresNeeded } = dying
  const row = (label: string, n: number, of: number, good: boolean) => (
    <div className="flex flex-wrap items-center gap-2">
      <span className="w-20 text-caption font-bold">{label}</span>
      {Array.from({ length: of }, (_, i) => (
        <span
          key={i}
          aria-hidden
          className={cn(
            'size-7 rounded-full',
            i < n
              ? good
                ? 'bg-ivory shadow-ivory-flat'
                : 'bg-stat-atk'
              : 'border-2 border-dashed border-line-strong',
          )}
        />
      ))}
    </div>
  )
  return (
    <div
      className="flex flex-col gap-1.5"
      role="img"
      aria-label={`${t('fight.death.successes')} ${successes}/${successesNeeded}, ${t('fight.death.failures')} ${failures}/${failuresNeeded}`}
    >
      {row(t('fight.death.successes'), successes, successesNeeded, true)}
      {row(t('fight.death.failures'), failures, failuresNeeded, false)}
    </div>
  )
}

/**
 * engine/save-against-death on the phone (planche « Mourir »): down at 0
 * but not dead; on my turn the whole turn is the save, the server rolls
 * and keeps the count. Three failures wait for the GM: nothing is said
 * before they decide.
 */
function DeathSaves({
  fight,
  dying,
  busy,
  onRoll,
}: {
  fight: FightView
  dying: Dying
  busy: boolean
  onRoll: () => void
}) {
  const { t } = useTranslation()
  const waiting = !dying.stable && dying.failures >= dying.failuresNeeded
  return (
    <section
      aria-label={t('fight.death.title')}
      className="flex flex-col gap-3 rounded-2xl border border-stat-atk bg-foe-deep p-4"
    >
      <h3 className="type-title text-[16px]">{fight.deathSave ? t('fight.death.turn') : t('fight.death.title')}</h3>
      {!dying.stable && !waiting && <p className="text-body text-chalk-soft">{t('fight.death.lead')}</p>}
      <SaveCircles dying={dying} />
      {dying.stable && <p className="text-body text-chalk-soft">{t('fight.death.stable')}</p>}
      {waiting && <p role="status" className="text-body text-chalk-soft">{t('fight.death.waiting')}</p>}
      {fight.deathSave && !waiting && (
        <CardButton title={t('fight.death.roll')} subtitle={t('fight.death.rollSub')} disabled={busy} onClick={onRoll} />
      )}
    </section>
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
              !f.party && 'bg-foe',
            )}
          >
            <span className="truncate font-bold">{f.name}</span>
            {f.hitPoints !== null && f.maxHitPoints !== null && (
              <Hearts hp={f.hitPoints} max={f.maxHitPoints} count={4} px={2} animated={false} />
            )}
            {f.conditions.length > 0 && <span className="text-mute-soft">{f.conditions.join(', ')}</span>}
            {f.dying && (
              <span className="text-stat-atk tabular-nums">
                {f.dying.stable
                  ? t('fight.death.stableShort')
                  : t('fight.death.short', { successes: f.dying.successes, failures: f.dying.failures })}
              </span>
            )}
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
