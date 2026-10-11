import { useCallback, useEffect, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Link, Navigate, useParams } from 'react-router'
import { Btn, Panel, field } from '@/features/gm/live/ui'
import { useLiveChanges } from '@/features/live/useLiveChanges'
import { MapCanvas } from '@/features/map/MapCanvas'
import { useImage } from '@/features/map/useImage'
import { useTileset } from '@/features/map/useTileset'
import { ApiError } from '@/lib/api'
import type { Cell, CellKind, MapData, TimeOfDay, TokenView, Weather } from '@/lib/board'
import { editMap, fetchMap, gmBackdropUrl, validateMap, type CampaignMap, type MapOp } from '@/lib/maps'
import { fetchGmMedia, gmImageUrl, type MediaList } from '@/lib/media'
import { undoRevision } from '@/lib/prep'

type Tool = 'floor' | 'wall' | 'water' | 'void' | 'door' | 'prop' | 'object' | 'light' | 'party' | 'foes' | 'erase'

const TOOLS: Tool[] = ['floor', 'wall', 'water', 'void', 'door', 'prop', 'object', 'light', 'party', 'foes', 'erase']
const BRUSHES: Tool[] = ['floor', 'wall', 'water', 'void']
const TIMES: TimeOfDay[] = ['dawn', 'day', 'dusk', 'night']
const WEATHERS: Weather[] = ['clear', 'cloudy', 'rain', 'storm', 'fog', 'snow', 'sandstorm']
const LIGHTS = ['', 'dark', 'dim', 'bright'] as const

type PageState =
  | { kind: 'loading' }
  | { kind: 'signed-out' }
  | { kind: 'not-found' }
  | { kind: 'error' }
  | { kind: 'ready'; saved: CampaignMap }

const isMap = (v: unknown): v is CampaignMap =>
  typeof v === 'object' && v !== null && 'map' in v && 'revision' in v

/**
 * `/campagnes/:campaignId/cartes/:mapId` — maps/edit-map-gm: the GM
 * paints the ground, walls, water and holes, places doors, decor, hidden
 * objects, lights and starts (on the map's layer or the secrets one),
 * sets the time and weather, then validates the map so it may be shown
 * at the table. An imported image stays behind the grid, the walls
 * veiled over it to trace them.
 *
 * maps/edit-maps-by-ops: each gesture is one batch of operations sent
 * at once on the revision the map was read at; the server's map is the
 * one shown. A batch refused as stale (someone — Claude — changed the
 * same cells meanwhile) reloads the fresh map and is replayed once on
 * it. What others write while the editor is open arrives live.
 */
export function MapEditorPage() {
  const { t } = useTranslation()
  const { campaignId = '', mapId = '' } = useParams()
  const [state, setState] = useState<PageState>({ kind: 'loading' })
  const [media, setMedia] = useState<MediaList | null>(null)
  const [tool, setTool] = useState<Tool>('wall')
  const [material, setMaterial] = useState('')
  const [prop, setProp] = useState('')
  const [label, setLabel] = useState('')
  const [layer, setLayer] = useState('base')
  const [name, setName] = useState<string | null>(null)
  const [notes, setNotes] = useState<string | null>(null)
  const [busy, setBusy] = useState(false)
  const [rebased, setRebased] = useState(false)
  const [error, setError] = useState<{
    code: string
    detail: string | null
  } | null>(null)
  /** The map as the server last answered it. */
  const current = useRef<CampaignMap | null>(null)
  /** Gestures not yet sent, in order. */
  const pending = useRef<MapOp[][]>([])
  const flushing = useRef(false)
  /** The revisions this editor wrote, newest last: what "undo" takes back. */
  const [mine, setMine] = useState<number[]>([])

  const adopt = useCallback((next: CampaignMap) => {
    current.current = next
    setState({ kind: 'ready', saved: next })
  }, [])

  useEffect(() => {
    let live = true
    Promise.all([fetchMap(campaignId, mapId), fetchGmMedia(campaignId)]).then(
      ([saved, m]) => {
        if (!live) return
        adopt(saved)
        setMedia(m)
      },
      (err: unknown) => {
        if (!live) return
        if (err instanceof ApiError && err.status === 401) setState({ kind: 'signed-out' })
        else if (err instanceof ApiError && err.status === 404) setState({ kind: 'not-found' })
        else setState({ kind: 'error' })
      },
    )
    return () => {
      live = false
    }
  }, [campaignId, mapId, adopt])

  const fail = (err: unknown) =>
    setError(err instanceof ApiError ? { code: err.code, detail: err.detail } : { code: 'action', detail: null })

  async function flush() {
    if (flushing.current) return
    flushing.current = true
    setBusy(true)
    while (pending.current.length > 0 && current.current) {
      const ops = pending.current.shift()!
      const send = (on: CampaignMap) => editMap(campaignId, mapId, on.revision, ops)
      try {
        let next: CampaignMap
        try {
          next = await send(current.current)
        } catch (err) {
          if (!(err instanceof ApiError && err.code === 'MAP_STALE' && isMap(err.fresh))) throw err
          // Someone changed the same cells: reload, and replay on it.
          adopt(err.fresh)
          setRebased(true)
          next = await send(err.fresh)
        }
        if (next.revision !== current.current.revision) setMine((m) => [...m, next.revision])
        adopt(next)
      } catch (err) {
        fail(err)
      }
    }
    flushing.current = false
    setBusy(false)
  }

  function gesture(ops: MapOp[]) {
    setError(null)
    setRebased(false)
    pending.current.push(ops)
    void flush()
  }

  // What Claude (or another screen) writes arrives on the GM-only `prep`
  // topic; a reply to the editor's own batch already carries it.
  useLiveChanges(campaignId, (topics) => {
    if (!topics.includes('prep') || flushing.current) return
    fetchMap(campaignId, mapId).then(
      (fresh) => {
        if (!flushing.current && fresh.revision > (current.current?.revision ?? -1)) adopt(fresh)
      },
      () => {},
    )
  })

  async function undo() {
    const last = mine[mine.length - 1]
    if (last === undefined) return
    setBusy(true)
    setError(null)
    try {
      await undoRevision(campaignId, last)
      setMine((m) => m.slice(0, -1))
      adopt(await fetchMap(campaignId, mapId))
    } catch (err) {
      fail(err)
    } finally {
      setBusy(false)
    }
  }

  async function validate() {
    setBusy(true)
    setError(null)
    try {
      adopt(await validateMap(campaignId, mapId))
    } catch (err) {
      fail(err)
    } finally {
      setBusy(false)
    }
  }

  const map: MapData | null = state.kind === 'ready' ? state.saved.map : null
  const { tileset, atlases } = useTileset(map, media, (id) => gmImageUrl(campaignId, id))
  const backdrop = useImage(map?.backdrop?.image ? gmBackdropUrl(campaignId, mapId) : null)

  if (state.kind === 'signed-out') return <Navigate to="/connexion" replace />
  const back = (
    <Link className="text-caption text-mute-soft underline underline-offset-4" to={`/campagnes/${campaignId}/atlas?carte=${encodeURIComponent(mapId)}`}>
      {t('maps.back')}
    </Link>
  )
  if (state.kind !== 'ready' || !map) {
    return (
      <main className="surface-table flex min-h-dvh flex-col gap-4 p-6 text-chalk">
        {back}
        {state.kind === 'loading' && <p role="status">{t('prep.loading')}</p>}
        {state.kind === 'not-found' && <p role="alert">{t('maps.notFound')}</p>}
        {state.kind === 'error' && <p role="alert">{t('prep.error')}</p>}
      </main>
    )
  }

  const { saved } = state
  const materials = Object.entries(tileset?.materials ?? {})
  const props = Object.entries(tileset?.props ?? {})
  const ground = material || materials[0]?.[0] || Object.values(map.grid.legend)[0]?.terrain || 'sol'
  const propKind = prop || props[0]?.[0] || 'caisse'

  function brush(cells: Cell[]) {
    const kind: CellKind =
      tool === 'wall'
        ? { terrain: ground, wall: true }
        : tool === 'water'
          ? { terrain: ground, water: 'deep' }
          : tool === 'void'
            ? { terrain: ground, void: true }
            : { terrain: ground }
    gesture([{ op: 'paint', cells, kind }])
  }

  function tap(c: Cell) {
    switch (tool) {
      case 'door':
        return gesture([{ op: 'door', at: c, layer }])
      case 'prop':
        return gesture([
          {
            op: 'place',
            at: c,
            thing: { kind: 'prop', prop: propKind, label: tileset?.props[propKind]?.name ?? propKind },
            layer,
          },
        ])
      case 'object':
        return gesture([{ op: 'place', at: c, thing: { kind: 'object', label: label.trim() }, layer }])
      case 'light':
        return gesture([{ op: 'place', at: c, thing: { kind: 'light' }, layer }])
      case 'party':
      case 'foes':
        return gesture([{ op: 'place', at: c, thing: { kind: 'start', side: tool }, layer }])
      case 'erase':
        return gesture([{ op: 'erase', at: c }])
      default:
        return gesture([{ op: 'paint', cells: [c], kind: { terrain: ground } }])
    }
  }

  const tokens: TokenView[] = (map.starts ?? []).map((s) => ({
    id: s.id,
    name: s.side === 'foes' ? t('maps.editor.foe') : t('maps.editor.hero'),
    at: s.at,
    party: s.side !== 'foes',
    mine: false,
    ghost: s.layer === 'secrets',
    // Starts are places, not characters: drawn as discs.
    look: null,
    facing: s.side === 'foes' ? 'west' : 'east',
    trail: [],
    moves: 0,
  }))
  const setAmbience = (patch: Partial<MapData['ambience']>) =>
    gesture([{ op: 'set', ambience: { ...map.ambience, ...patch } }])
  const commitName = () => {
    if (name !== null && name.trim() !== map.name) gesture([{ op: 'set', name }])
    setName(null)
  }
  const commitNotes = () => {
    if (notes !== null && notes !== (map.gm_notes ?? '')) gesture([{ op: 'set', notes }])
    setNotes(null)
  }

  return (
    <main className="surface-table flex min-h-dvh flex-col gap-4 p-5 text-chalk">
      {back}
      <div className="flex flex-wrap items-baseline gap-3">
        <input
          className={`${field} type-title text-[16px]`}
          value={name ?? map.name}
          maxLength={80}
          onChange={(e) => setName(e.target.value)}
          onBlur={commitName}
          onKeyDown={(e) => {
            if (e.key === 'Enter') e.currentTarget.blur()
          }}
          aria-label={t('maps.editor.name')}
        />
        <span className="text-caption text-mute-soft">
          {saved.validatedAt ? t('maps.validated') : t('maps.draft')}
        </span>
        <span className="text-caption text-mute-soft" role="status">
          {busy ? t('maps.editor.saving') : t('maps.editor.saved')}
        </span>
      </div>
      {rebased && (
        <p role="status" className="text-body text-mute-soft">
          {t('maps.editor.rebased')}
        </p>
      )}
      {error && (
        <p role="alert" className="text-body text-stat-atk">
          {t(`maps.errors.${error.code}`, {
            defaultValue: t('prep.generate.errors.action'),
          })}
          {(error.code === 'MAP_INVALID' || error.code === 'MAP_OP_INVALID') && error.detail && <span className="block text-caption">{error.detail}</span>}
        </p>
      )}

      <div className="grid gap-4 xl:grid-cols-[300px_minmax(0,1fr)]">
        <div className="flex flex-col gap-3">
          <Panel title={t('maps.editor.tools')}>
            <div className="flex flex-wrap gap-1.5" role="radiogroup" aria-label={t('maps.editor.tools')}>
              {TOOLS.map((k) => (
                <Btn key={k} main={tool === k} role="radio" aria-checked={tool === k} onClick={() => setTool(k)}>
                  {t(`maps.editor.tool.${k}`)}
                </Btn>
              ))}
            </div>
            {(BRUSHES.includes(tool) || tool === 'door') && materials.length > 0 && (
              <select
                className={field}
                value={ground}
                onChange={(e) => setMaterial(e.target.value)}
                aria-label={t('maps.editor.material')}
              >
                {materials.map(([id, m]) => (
                  <option key={id} value={id}>
                    {m.name}
                  </option>
                ))}
              </select>
            )}
            {tool === 'prop' && props.length > 0 && (
              <select
                className={field}
                value={propKind}
                onChange={(e) => setProp(e.target.value)}
                aria-label={t('maps.editor.prop')}
              >
                {props.map(([id, p]) => (
                  <option key={id} value={id}>
                    {p.name}
                  </option>
                ))}
              </select>
            )}
            {tool === 'object' && (
              <input
                className={field}
                value={label}
                onChange={(e) => setLabel(e.target.value)}
                placeholder={t('maps.editor.objectLabel')}
                aria-label={t('maps.editor.objectLabel')}
              />
            )}
            <label className="flex items-center gap-2 text-caption">
              <input
                type="checkbox"
                checked={layer === 'secrets'}
                onChange={(e) => setLayer(e.target.checked ? 'secrets' : 'base')}
              />
              {t('maps.editor.secret')}
            </label>
            <p className="text-caption text-mute-soft">
              {BRUSHES.includes(tool) ? t('maps.editor.brushHint') : t('maps.editor.tapHint')}
            </p>
          </Panel>

          <Panel title={t('maps.editor.ambience')}>
            <select
              className={field}
              value={map.ambience.time ?? 'day'}
              onChange={(e) => setAmbience({ time: e.target.value as TimeOfDay })}
              aria-label={t('gmLive.board.time')}
            >
              {TIMES.map((k) => (
                <option key={k} value={k}>
                  {t(`gmLive.board.times.${k}`)}
                </option>
              ))}
            </select>
            <select
              className={field}
              value={map.ambience.weather ?? 'clear'}
              onChange={(e) => setAmbience({ weather: e.target.value as Weather })}
              aria-label={t('gmLive.board.weather')}
            >
              {WEATHERS.map((k) => (
                <option key={k} value={k}>
                  {t(`map.weather.${k}`)}
                </option>
              ))}
            </select>
            <select
              className={field}
              value={map.ambience.light ?? ''}
              onChange={(e) =>
                setAmbience({
                  light: e.target.value === '' ? null : (e.target.value as 'dark' | 'dim' | 'bright'),
                })
              }
              aria-label={t('maps.editor.light')}
            >
              {LIGHTS.map((k) => (
                <option key={k} value={k}>
                  {t(`maps.editor.lights.${k || 'time'}`)}
                </option>
              ))}
            </select>
            <textarea
              className={field}
              value={notes ?? map.gm_notes ?? ''}
              onChange={(e) => setNotes(e.target.value)}
              onBlur={commitNotes}
              placeholder={t('maps.editor.notes')}
              aria-label={t('maps.editor.notes')}
              rows={3}
            />
          </Panel>

          <div className="flex flex-wrap gap-2">
            <Btn disabled={busy || mine.length === 0} onClick={() => void undo()}>
              {t('maps.editor.undo')}
            </Btn>
            <Btn main disabled={busy || Boolean(saved.validatedAt)} onClick={() => void validate()}>
              {t('maps.editor.validate')}
            </Btn>
          </div>
          <p className="text-caption text-mute-soft">{t('maps.editor.validateHint')}</p>
        </div>

        <MapCanvas
          scene={{
            map,
            tileset,
            atlases,
            backdrop,
            showWalls: true,
            tokens,
            highlight: (map.lights ?? []).map((l) => l.at),
          }}
          className="max-h-[75vh]"
          onCell={BRUSHES.includes(tool) ? undefined : tap}
          onPaint={BRUSHES.includes(tool) ? brush : undefined}
        />
      </div>
    </main>
  )
}
