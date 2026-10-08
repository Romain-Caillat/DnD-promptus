import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Link, Navigate, useParams } from 'react-router'
import { Btn, Panel, field } from '@/features/gm/live/ui'
import { MapCanvas } from '@/features/map/MapCanvas'
import { useImage } from '@/features/map/useImage'
import { useTileset } from '@/features/map/useTileset'
import { ApiError } from '@/lib/api'
import type { Cell, MapData, TimeOfDay, TokenView, Weather } from '@/lib/board'
import { fetchMap, gmBackdropUrl, saveMap, validateMap, type CampaignMap } from '@/lib/maps'
import { fetchGmMedia, gmImageUrl, type MediaList } from '@/lib/media'
import { erase, paint, place, toggleDoor } from './edit'

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

/**
 * `/campagnes/:campaignId/cartes/:mapId` — maps/edit-map-gm: the GM
 * paints the ground, walls, water and holes, places doors, decor, hidden
 * objects, lights and starts (on the map's layer or the secrets one),
 * sets the time and weather, saves, then validates the map so it may be
 * shown at the table. An imported image stays behind the grid, the
 * walls veiled over it to trace them.
 */
export function MapEditorPage() {
  const { t } = useTranslation()
  const { campaignId = '', mapId = '' } = useParams()
  const [state, setState] = useState<PageState>({ kind: 'loading' })
  const [map, setMap] = useState<MapData | null>(null)
  const [media, setMedia] = useState<MediaList | null>(null)
  const [tool, setTool] = useState<Tool>('wall')
  const [material, setMaterial] = useState('')
  const [prop, setProp] = useState('')
  const [label, setLabel] = useState('')
  const [layer, setLayer] = useState('base')
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<{
    code: string
    detail: string | null
  } | null>(null)

  useEffect(() => {
    let live = true
    Promise.all([fetchMap(campaignId, mapId), fetchGmMedia(campaignId)]).then(
      ([saved, m]) => {
        if (!live) return
        setState({ kind: 'ready', saved })
        setMap(saved.map)
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
  }, [campaignId, mapId])

  const { tileset, atlases } = useTileset(map, media, (id) => gmImageUrl(campaignId, id))
  const backdrop = useImage(map?.backdrop?.image ? gmBackdropUrl(campaignId, mapId) : null)

  if (state.kind === 'signed-out') return <Navigate to="/connexion" replace />
  const back = (
    <Link className="text-caption text-mute-soft underline underline-offset-4" to={`/campagnes/${campaignId}/cartes`}>
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
  const dirty = JSON.stringify(map) !== JSON.stringify(saved.map)
  const materials = Object.entries(tileset?.materials ?? {})
  const props = Object.entries(tileset?.props ?? {})
  const ground = material || materials[0]?.[0] || Object.values(map.grid.legend)[0]?.terrain || 'sol'
  const propKind = prop || props[0]?.[0] || 'caisse'

  function brush(cells: Cell[]) {
    const kind =
      tool === 'wall'
        ? { terrain: ground, wall: true }
        : tool === 'water'
          ? { terrain: ground, water: 'deep' as const }
          : tool === 'void'
            ? { terrain: ground, void: true }
            : { terrain: ground }
    setMap((m) => (m ? paint(m, cells, kind) : m))
  }

  function tap(c: Cell) {
    setMap((m) => {
      if (!m) return m
      switch (tool) {
        case 'door':
          return toggleDoor(m, c, layer)
        case 'prop':
          return place(
            m,
            c,
            {
              kind: 'prop',
              prop: propKind,
              label: tileset?.props[propKind]?.name ?? propKind,
            },
            layer,
          )
        case 'object':
          return place(m, c, { kind: 'object', label: label.trim() }, layer)
        case 'light':
          return place(m, c, { kind: 'light' }, layer)
        case 'party':
        case 'foes':
          return place(m, c, { kind: 'start', side: tool }, layer)
        case 'erase':
          return erase(m, c)
        default:
          return paint(m, [c], { terrain: ground })
      }
    })
  }

  async function act(run: () => Promise<CampaignMap>) {
    setBusy(true)
    setError(null)
    try {
      const next = await run()
      setState({ kind: 'ready', saved: next })
      setMap(next.map)
    } catch (err) {
      setError(err instanceof ApiError ? { code: err.code, detail: err.detail } : { code: 'action', detail: null })
    } finally {
      setBusy(false)
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
    setMap((m) => (m ? { ...m, ambience: { ...m.ambience, ...patch } } : m))

  return (
    <main className="surface-table flex min-h-dvh flex-col gap-4 p-5 text-chalk">
      {back}
      <div className="flex flex-wrap items-baseline gap-3">
        <input
          className={`${field} type-title text-[16px]`}
          value={map.name}
          maxLength={80}
          onChange={(e) => setMap({ ...map, name: e.target.value })}
          aria-label={t('maps.editor.name')}
        />
        <span className="text-caption text-mute-soft">
          {saved.validatedAt && !dirty ? t('maps.validated') : t('maps.draft')}
        </span>
      </div>
      {error && (
        <p role="alert" className="text-body text-stat-atk">
          {t(`maps.errors.${error.code}`, {
            defaultValue: t('prep.generate.errors.action'),
          })}
          {error.code === 'MAP_INVALID' && error.detail && <span className="block text-caption">{error.detail}</span>}
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
              value={map.gm_notes ?? ''}
              onChange={(e) => setMap({ ...map, gm_notes: e.target.value || null })}
              placeholder={t('maps.editor.notes')}
              aria-label={t('maps.editor.notes')}
              rows={3}
            />
          </Panel>

          <div className="flex flex-wrap gap-2">
            <Btn main disabled={busy || !dirty} onClick={() => void act(() => saveMap(campaignId, map))}>
              {t('maps.editor.save')}
            </Btn>
            <Btn disabled={busy || !dirty} onClick={() => setMap(saved.map)}>
              {t('maps.editor.undo')}
            </Btn>
            <Btn
              main
              disabled={busy || dirty || Boolean(saved.validatedAt)}
              onClick={() => void act(() => validateMap(campaignId, mapId))}
            >
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
