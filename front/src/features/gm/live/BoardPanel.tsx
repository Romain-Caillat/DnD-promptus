import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { eventLine } from '@/features/map/events'
import { MapCanvas } from '@/features/map/MapCanvas'
import { cellKey } from '@/features/map/render'
import { readGrid } from '@/features/map/render'
import { useImage } from '@/features/map/useImage'
import { useTileset } from '@/features/map/useTileset'
import type { Cell, Edit, GmBoard, GmCommand, TokenView, Weather, TimeOfDay } from '@/lib/board'
import { gmBackdropUrl } from '@/lib/maps'
import { DeathDecisions } from './DeathDecisions'
import { gmImageUrl, type MediaList } from '@/lib/media'
import { cn } from '@/lib/utils'
import { Btn, Panel, field } from './ui'

const TOOLS = ['move', 'reveal', 'hide'] as const
type Tool = (typeof TOOLS)[number]

const WEATHERS: Weather[] = ['clear', 'cloudy', 'rain', 'storm', 'fog', 'snow', 'sandstorm']
const TIMES: TimeOfDay[] = ['dawn', 'day', 'dusk', 'night']

/**
 * The grid from the GM's side (gm/run-live-screen, maps/reveal-fog-and-
 * hidden, gm/run-combat): the whole map with what is hidden, the fog
 * veiled rather than black, painted away or back with a drag; tokens
 * moved by tap, hidden or made invisible; doors, weather and time. Then
 * the fight: start an encounter of the story, the order with every hit
 * point, the co-GM's proposal for the adversary's turn (played on a copy,
 * accepted as is), conditions, stop — and the loot to hand out.
 */
export function BoardPanel({
  campaignId,
  data,
  media,
  live,
  onShow,
  onEdit,
  onStart,
  onCommand,
  onLoot,
}: {
  campaignId: string
  data: GmBoard
  media: MediaList | null
  live: boolean
  onShow: (map: string) => void
  onEdit: (e: Edit) => void
  onStart: (node: string) => void
  onCommand: (c: GmCommand) => void
  onLoot: (index: number, character: string) => void
}) {
  const { t } = useTranslation()
  const [tool, setTool] = useState<Tool>('move')
  const [selected, setSelected] = useState<string | null>(null)
  const board = data.board
  const { tileset, atlases } = useTileset(board?.map ?? null, media, (id) => gmImageUrl(campaignId, id))
  const backdrop = useImage(board?.map.backdrop?.image ? gmBackdropUrl(campaignId, board.mapId) : null)
  const enc = data.encounter
  const fighting = Boolean(enc?.live)

  if (!board) {
    return (
      <Panel title={t('gmLive.board.title')}>
        <p className="text-caption text-mute">{t('gmLive.board.none')}</p>
        <MapPicker data={data} live={live} onShow={onShow} />
      </Panel>
    )
  }

  const grid = readGrid(board.map)
  const revealed = new Set(board.revealed.map(cellKey))
  const veiled = new Set<string>()
  if (board.fog) {
    for (let y = 0; y < grid.height; y++) for (let x = 0; x < grid.width; x++) if (!revealed.has(`${x},${y}`)) veiled.add(`${x},${y}`)
  }
  const tokens: TokenView[] = board.tokens.map((tk) => ({
    id: tk.id,
    name: tk.name,
    at: tk.at,
    party: tk.kind === 'character',
    mine: false,
    ghost: tk.hidden || tk.invisible,
  }))
  const sel = board.tokens.find((tk) => tk.id === selected)

  function tap(cell: Cell) {
    const on = board!.tokens.find((tk) => tk.at[0] === cell[0] && tk.at[1] === cell[1])
    if (on) {
      setSelected(on.id === selected ? null : on.id)
      return
    }
    if (sel && !fighting) {
      onEdit({ kind: 'moveToken', token: sel.id, at: cell })
      setSelected(null)
    }
  }

  return (
    <Panel
      title={t('gmLive.board.title')}
      actions={<MapPicker data={data} live={live && !fighting} onShow={onShow} current={board.mapId} />}
    >
      <div className="flex flex-wrap items-center gap-1.5">
        {TOOLS.map((x) => (
          <Btn key={x} main={tool === x} onClick={() => setTool(x)}>
            {t(`gmLive.board.tool.${x}`)}
          </Btn>
        ))}
        <label className="flex items-center gap-1 text-caption">
          <input type="checkbox" checked={board.fog} onChange={(e) => onEdit({ kind: 'fog', enabled: e.target.checked })} />
          {t('gmLive.board.fog')}
        </label>
        <select
          className={field}
          value={board.map.ambience.weather ?? 'clear'}
          onChange={(e) => onEdit({ kind: 'ambience', weather: e.target.value as Weather })}
          aria-label={t('gmLive.board.weather')}
        >
          {WEATHERS.map((w) => (
            <option key={w} value={w}>
              {t(`map.weather.${w}`)}
            </option>
          ))}
        </select>
        <select
          className={field}
          value={board.map.ambience.time ?? 'day'}
          onChange={(e) => onEdit({ kind: 'ambience', time: e.target.value as TimeOfDay })}
          aria-label={t('gmLive.board.time')}
        >
          {TIMES.map((x) => (
            <option key={x} value={x}>
              {t(`gmLive.board.times.${x}`)}
            </option>
          ))}
        </select>
      </div>
      <MapCanvas
        scene={{
          map: board.map,
          tileset,
          atlases,
          backdrop,
          tokens,
          veiled,
          selected,
          reachable: fighting ? (enc?.reachable ?? []) : [],
        }}
        className="max-h-[60vh]"
        onCell={tool === 'move' ? tap : undefined}
        onPaint={
          tool === 'move'
            ? undefined
            : (cells) => onEdit(tool === 'reveal' ? { kind: 'revealCells', cells } : { kind: 'hideCells', cells })
        }
      />
      {board.map.gm_notes && <p className="text-caption text-mute-soft">{board.map.gm_notes}</p>}
      {sel && (
        <div className="flex flex-wrap items-center gap-1.5 text-caption">
          <span className="font-bold">{sel.name}</span>
          <span className="text-mute-soft">{t('gmLive.board.moveHint')}</span>
          <Btn onClick={() => onEdit({ kind: 'tokenState', token: sel.id, hidden: !sel.hidden })}>
            {t(sel.hidden ? 'gmLive.board.showToken' : 'gmLive.board.hideToken')}
          </Btn>
          <Btn onClick={() => onEdit({ kind: 'tokenState', token: sel.id, invisible: !sel.invisible })}>
            {t(sel.invisible ? 'gmLive.board.visible' : 'gmLive.board.invisible')}
          </Btn>
          {sel.kind === 'npc' && !fighting && (
            <Btn onClick={() => onEdit({ kind: 'removeToken', token: sel.id })}>{t('gmLive.board.remove')}</Btn>
          )}
        </div>
      )}
      {(board.map.doors ?? []).length > 0 && (
        <div className="flex flex-wrap gap-1.5">
          {(board.map.doors ?? []).map((d) => (
            <Btn
              key={d.id}
              onClick={() => onEdit({ kind: 'door', door: d.id, state: d.state === 'open' ? 'closed' : 'open' })}
            >
              {t(d.state === 'open' ? 'gmLive.board.closeDoor' : 'gmLive.board.openDoor', { door: d.label ?? d.id })}
            </Btn>
          ))}
        </div>
      )}
      {(board.map.layers ?? []).filter((l) => l.visibility === 'gm').length > 0 && (
        <div className="flex flex-wrap gap-1.5">
          {(board.map.layers ?? [])
            .filter((l) => l.visibility === 'gm')
            .map((l) => (
              <Btn key={l.id} onClick={() => onEdit({ kind: 'revealLayer', layer: l.id })}>
                {t('gmLive.board.revealLayer', { layer: l.name })}
              </Btn>
            ))}
        </div>
      )}
      <FightBlock data={data} live={live} onStart={onStart} onCommand={onCommand} onLoot={onLoot} />
    </Panel>
  )
}

function MapPicker({
  data,
  live,
  onShow,
  current,
}: {
  data: GmBoard
  live: boolean
  onShow: (map: string) => void
  current?: string
}) {
  const { t } = useTranslation()
  if (data.maps.length === 0) return <span className="text-caption text-mute">{t('gmLive.board.noMaps')}</span>
  return (
    <div className="flex flex-wrap gap-1">
      {data.maps.map((m) => (
        <Btn key={m.id} main={m.id === current} disabled={!live || m.id === current} onClick={() => onShow(m.id)}>
          {t('gmLive.board.show', { name: m.name })}
        </Btn>
      ))}
    </div>
  )
}

function FightBlock({
  data,
  live,
  onStart,
  onCommand,
  onLoot,
}: {
  data: GmBoard
  live: boolean
  onStart: (node: string) => void
  onCommand: (c: GmCommand) => void
  onLoot: (index: number, character: string) => void
}) {
  const { t } = useTranslation()
  const enc = data.encounter
  const [who, setWho] = useState('')
  const [condition, setCondition] = useState('')
  const characters = (data.board?.tokens ?? []).filter((tk) => tk.kind === 'character')

  if (!enc?.live) {
    return (
      <div className="flex flex-col gap-1.5 border-t border-line pt-2">
        <span className="type-label">{t('gmLive.fight.title')}</span>
        <div className="flex flex-wrap gap-1.5">
          {data.encounters.map((e) => (
            <Btn key={e.node} disabled={!live} onClick={() => onStart(e.node)}>
              {t('gmLive.fight.start', { title: e.title })}
            </Btn>
          ))}
        </div>
        {enc && <DeathDecisions enc={enc} onCommand={onCommand} />}
        {enc && enc.loot.length > 0 && (
          <div className="flex flex-col gap-1">
            <span className="type-label">{t('gmLive.fight.loot')}</span>
            {enc.loot.map((l) => (
              <div key={l.index} className="flex items-center justify-between gap-2 text-caption">
                <span className={cn(l.givenTo && 'text-mute line-through')}>
                  {l.name}
                  {l.hidden && ` · ${t('gmLive.fight.hiddenLoot')}`}
                  {l.found && <span className="text-mute-soft"> · {l.found}</span>}
                </span>
                {!l.givenTo && (
                  <select
                    className={field}
                    value=""
                    onChange={(e) => e.target.value && onLoot(l.index, e.target.value)}
                    aria-label={t('gmLive.fight.giveTo', { name: l.name })}
                  >
                    <option value="">{t('gmLive.fight.giveTo', { name: '' })}</option>
                    {characters.map((c) => (
                      <option key={c.ref} value={c.ref}>
                        {c.name}
                      </option>
                    ))}
                  </select>
                )}
              </div>
            ))}
          </div>
        )}
      </div>
    )
  }

  const f = enc.fight
  const name = (id: string) => f.scene.combatants[id]?.name ?? id
  const active = f.order[f.turn]
  const foeTurn = f.scene.combatants[active]?.side === 'opposition'
  const lines = (enc.proposal?.events ?? enc.events)
    .map((e) => eventLine(e, name, t))
    .filter((l): l is string => l !== null)
    .slice(-10)
  return (
    <div className="flex flex-col gap-2 border-t border-line pt-2">
      <div className="flex items-center justify-between">
        <span className="type-label">{t('gmLive.fight.round', { round: f.round, who: name(active) })}</span>
        <Btn onClick={() => onCommand({ kind: 'stop' })}>{t('gmLive.fight.stop')}</Btn>
      </div>
      <ol className="flex flex-col gap-1">
        {f.order.map((id) => {
          const c = f.scene.combatants[id]
          if (!c) return null
          return (
            <li
              key={id}
              className={cn(
                'flex items-center justify-between gap-2 rounded-md px-2 py-1 text-caption',
                id === active ? 'border border-ivory' : 'border border-transparent',
                f.standing[id] !== 'in_fight' && 'opacity-40',
                c.side === 'opposition' && 'bg-[#3a1512]',
              )}
            >
              <span className="font-bold">{c.name}</span>
              <span className="tabular-nums">
                {f.standing[id] === 'dead'
                  ? t('gmLive.fight.dead')
                  : t('gmLive.fight.hp', { hp: c.hit_points, max: enc.maxHitPoints[id] ?? '?' })}
                {c.conditions.length > 0 && ` · ${c.conditions.map((x) => x.name).join(', ')}`}
                {f.dying?.[id] &&
                  ` · ${t('gmLive.fight.saves', { successes: f.dying[id].successes, failures: f.dying[id].failures })}`}
              </span>
            </li>
          )
        })}
      </ol>
      <DeathDecisions enc={enc} onCommand={onCommand} />
      {enc.deathSaveDue && (
        <div className="flex flex-wrap items-center gap-1.5">
          <span className="text-caption">{t('gmLive.fight.saveDue', { who: name(active) })}</span>
          <Btn onClick={() => onCommand({ kind: 'deathSave' })}>{t('gmLive.fight.rollForThem')}</Btn>
        </div>
      )}
      {foeTurn && (
        <div className="flex flex-wrap gap-1.5">
          <Btn main={!enc.proposal} onClick={() => onCommand({ kind: 'propose' })}>
            {t('gmLive.fight.propose')}
          </Btn>
          {enc.proposal && (
            <Btn main onClick={() => onCommand({ kind: 'accept' })}>
              {t('gmLive.fight.accept')}
            </Btn>
          )}
          <Btn onClick={() => onCommand({ kind: 'adversary', command: { kind: 'endTurn' } })}>
            {t('gmLive.fight.endTurn')}
          </Btn>
        </div>
      )}
      {enc.proposal && <p className="type-label">{t('gmLive.fight.proposal')}</p>}
      <ul className="flex flex-col gap-0.5 text-caption text-chalk-soft">
        {lines.map((l, i) => (
          <li key={i}>{l}</li>
        ))}
      </ul>
      {data.conditions && data.conditions.length > 0 && (
        <div className="flex flex-wrap items-center gap-1.5">
          <select className={field} value={who} onChange={(e) => setWho(e.target.value)} aria-label={t('gmLive.fight.who')}>
            <option value="">{t('gmLive.fight.who')}</option>
            {f.order.map((id) => (
              <option key={id} value={id}>
                {name(id)}
              </option>
            ))}
          </select>
          <select
            className={field}
            value={condition}
            onChange={(e) => setCondition(e.target.value)}
            aria-label={t('gmLive.fight.condition')}
          >
            <option value="">{t('gmLive.fight.condition')}</option>
            {data.conditions.map((c) => (
              <option key={c.id} value={c.id}>
                {c.name}
              </option>
            ))}
          </select>
          <Btn disabled={!who || !condition} onClick={() => onCommand({ kind: 'condition', who, condition, remove: false })}>
            {t('gmLive.fight.addCondition')}
          </Btn>
          <Btn disabled={!who || !condition} onClick={() => onCommand({ kind: 'condition', who, condition, remove: true })}>
            {t('gmLive.fight.removeCondition')}
          </Btn>
        </div>
      )}
    </div>
  )
}

