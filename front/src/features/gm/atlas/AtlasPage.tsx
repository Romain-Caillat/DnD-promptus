import { useEffect, useMemo, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Link, Navigate, useParams, useSearchParams } from 'react-router'
import { Btn, Panel, TouchScreen, field } from '@/features/gm/live/ui'
import { useLiveChanges } from '@/features/live/useLiveChanges'
import { MapCanvas } from '@/features/map/MapCanvas'
import { gmMarks } from '@/features/map/pins'
import { useImage } from '@/features/map/useImage'
import { useTileset } from '@/features/map/useTileset'
import { HexMap } from '@/features/travel/HexMap'
import { ApiError } from '@/lib/api'
import type { Cell } from '@/lib/board'
import { fetchAtlas, generateMap, gmBackdropUrl, type Atlas, type AtlasMap, type Pin } from '@/lib/maps'
import { applyEdits, type Edit } from '@/lib/prep'
import { fetchGmMedia, gmImageUrl, type MediaList } from '@/lib/media'
import { TABLET_QUERY, useMediaQuery } from '@/lib/useMediaQuery'
import { cn } from '@/lib/utils'
import { canLeave, climb, depth, doorAt, doorCells, enter, index, leave, switchable, trail, withoutLayers } from './atlas'
import { CastSection } from './CastSection'
import { STORY_LAYERS, castMarks, placeEdit, readKey, removeEdit, type StoryLayer, type Target } from './cast'

type PageState =
  | { kind: 'loading' }
  | { kind: 'signed-out' }
  | { kind: 'not-found' }
  | { kind: 'error' }
  | { kind: 'ready'; atlas: Atlas }

/** The three scales, as the breadcrumb names them. */
type ScaleName = 'world' | 'place' | 'encounter'
const scaleOf = (m: AtlasMap): ScaleName => m.map.scale ?? 'encounter'

/** What has no place on a map, and the preparation tab that holds it. */
const DRAWER = ['bible', 'graph', 'sheets'] as const

/**
 * `/campagnes/:campaignId/atlas` — maps/browse-atlas and the shell of
 * gm/prep-on-atlas: every map of the world and the campaign, nested by
 * their exits (region in hexes › place › encounter). The GM enters a
 * place by tapping where it opens on its parent (or from the side
 * panel), and zooming out goes back where they came from. The
 * breadcrumb, the map's own layers, the side panel and the strip of
 * every map at the bottom frame it; a drawer leads to what has no place
 * (bible, scenes and acts, sheets). On a tablet the map is handled
 * with the fingers and the side panel folds away.
 *
 * The scenes stand on the maps as numbered pins (campaign/pin-scenes-
 * on-maps), carried by their places in the story: tapping one lists its
 * scenes, each opening the scene sheet; the pin tool anchors a place on
 * a cell, moves or removes it — a story edit, so a line of the journal.
 * A scene can generate its fight map from there (maps/generate-map-llm).
 *
 * NPCs, clues and fights are placed the same way (campaign/place-cast-
 * and-clues): round pieces, diamonds and crossed squares on the GM's
 * layer, set down, moved or taken off by story edits; an NPC for every
 * scene on the map or for one. The four story layers — Histoire (the
 * pins), Personnages, Combats, Indices — light or dim apart from the
 * map's own layers. A fight opens its battle plan and says how many
 * opponents it already places.
 *
 * GM only: a player receives the one map on the table, through the
 * board's projection, never the atlas.
 */
export function AtlasPage() {
  const { t } = useTranslation()
  const { campaignId = '' } = useParams()
  const [params, setParams] = useSearchParams()
  const [state, setState] = useState<PageState>({ kind: 'loading' })
  const [version, setVersion] = useState(0)
  const [media, setMedia] = useState<MediaList | null>(null)
  /** The maps walked through, the last shown; seeded from `?carte=`. */
  const [walk, setWalk] = useState<string[]>(() => {
    const at = params.get('carte')
    return at ? [at] : []
  })
  const [motion, setMotion] = useState<'in' | 'out' | null>(null)
  const [hidden, setHidden] = useState<ReadonlySet<string>>(new Set())
  const [drawer, setDrawer] = useState(false)
  const tablet = useMediaQuery(TABLET_QUERY)
  const [panel, setPanel] = useState(true)
  /** The mark chosen (a place's pin, or a cast key), and what the next tap sets down. */
  const [picked, setPicked] = useState<string | null>(null)
  const [placing, setPlacing] = useState<Target | null>(null)
  const [lit, setLit] = useState<ReadonlySet<StoryLayer>>(new Set(STORY_LAYERS))
  const [choice, setChoice] = useState('')
  const [busy, setBusy] = useState(false)
  const [failed, setFailed] = useState<string | null>(null)

  useEffect(() => {
    let live = true
    fetchAtlas(campaignId).then(
      (atlas) => {
        if (live) setState({ kind: 'ready', atlas })
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
  }, [campaignId, version])

  useEffect(() => {
    let live = true
    fetchGmMedia(campaignId).then(
      (m) => {
        if (live) setMedia(m)
      },
      () => {},
    )
    return () => {
      live = false
    }
  }, [campaignId])

  // A map added, changed or removed (the GM elsewhere, Claude) moves the
  // GM-only `prep` topic.
  useLiveChanges(campaignId, (topics) => {
    if (topics.includes('prep')) setVersion((v) => v + 1)
  })

  const atlas = state.kind === 'ready' ? state.atlas : null
  const maps = useMemo(() => (atlas ? index(atlas) : new Map<string, AtlasMap>()), [atlas])
  // The map shown: the last walked that still exists, else the first root.
  const shownId = [...walk].reverse().find((id) => maps.has(id)) ?? atlas?.roots[0] ?? null
  const entry = shownId ? (maps.get(shownId) ?? null) : null
  const map = entry ? withoutLayers(entry.map, hidden) : null
  const { tileset, atlases } = useTileset(entry?.map ?? null, media, (id) => gmImageUrl(campaignId, id))
  const backdrop = useImage(entry?.backdrop && entry.map.backdrop?.image ? gmBackdropUrl(campaignId, entry.map.id) : null)

  if (state.kind === 'signed-out') return <Navigate to="/connexion" replace />
  const back = (
    <Link className="text-caption text-mute-soft underline underline-offset-4" to={`/campagnes/${campaignId}`}>
      {t('prep.back')}
    </Link>
  )
  if (state.kind !== 'ready') {
    return (
      <main className="surface-table flex min-h-dvh flex-col gap-4 p-6 text-chalk">
        {back}
        {state.kind === 'loading' && <p role="status">{t('prep.loading')}</p>}
        {state.kind === 'not-found' && <p role="alert">{t('prep.notFound')}</p>}
        {state.kind === 'error' && <p role="alert">{t('prep.error')}</p>}
      </main>
    )
  }

  const current = walk.filter((id) => maps.has(id))
  const here = current.length > 0 ? current : shownId ? [shownId] : []

  function go(next: string[], how: 'in' | 'out' | null) {
    setWalk(next)
    setMotion(how)
    setHidden(new Set())
    setPicked(null)
    setPlacing(null)
    const last = next[next.length - 1]
    if (last) setParams({ carte: last }, { replace: true })
  }
  const open = (id: string) => go(enter(here, id), 'in')
  const jump = (id: string) => go(enter(here, id), null)
  const out = () => go(leave(here, maps), 'out')
  /** One story edit placing something (pin, NPC, clue, fight), then the atlas again. */
  async function send(edit: Edit) {
    setBusy(true)
    setFailed(null)
    try {
      await applyEdits(campaignId, [edit])
      setVersion((v) => v + 1)
    } catch (err) {
      setFailed(err instanceof ApiError ? err.code : 'action')
    } finally {
      setBusy(false)
    }
  }
  /** maps/generate-map-llm, from a pin: the scene's fight map, attached to it (as the scene sheet does). */
  async function generate(scene: string) {
    setBusy(true)
    setFailed(null)
    try {
      const made = await generateMap(campaignId, scene)
      await applyEdits(campaignId, [{ op: 'set', target: scene, field: 'map', value: made.map.id }])
      setVersion((v) => v + 1)
      go(enter(here, made.map.id), 'in')
    } catch (err) {
      setFailed(err instanceof ApiError ? err.code : 'action')
    } finally {
      setBusy(false)
    }
  }
  const tap = (cell: Cell) => {
    if (!entry) return
    if (placing) {
      const target = placing
      setPlacing(null)
      if (target.kind === 'place') setPicked(target.id)
      void send(placeEdit(state.atlas.castable, target, entry.map.id, cell))
      return
    }
    const to = doorAt(entry, cell)
    if (to) open(to)
  }
  const pick = (place: string) => setPicked((p) => (p === place ? null : place))

  const crumbs = entry ? trail(maps, entry.map.id) : []
  const name = (id: string) => maps.get(id)?.map.name ?? id
  const layers = entry ? switchable(entry.map) : []
  const marks = entry ? doorCells(entry) : []
  const pins: Pin[] = entry?.pins ?? []
  const pinTitle = (p: Pin) =>
    t('atlas.pins.pin', { name: p.name, scenes: p.scenes.map((x) => `${x.number}. ${x.title}`).join(', ') })
  const sceneName = (id: string) => {
    const sc = state.atlas.castable.scenes.find((x) => x.id === id)
    return sc ? t('atlas.pins.scene', { number: sc.number, title: sc.title }) : id
  }
  const pinMarks = [
    ...(lit.has('story') ? gmMarks(pins, pinTitle, { picked }) : []),
    ...(entry
      ? castMarks(
          entry.cast,
          lit,
          {
            npc: (name, scene) => (scene ? t('atlas.cast.npcFor', { name, scene: sceneName(scene) }) : name),
            clue: (text) => t('atlas.cast.clue', { text }),
            fight: (number, title) => t('atlas.cast.fight', { number, title }),
          },
          picked,
        )
      : []),
  ]
  const chosen = pins.find((p) => p.location === picked) ?? null
  const castPicked = picked ? readKey(picked) : null
  const pinning = placing?.kind === 'place' ? placing.id : null
  const places = state.atlas.places
  const placeName = (id: string) => places.find((p) => p.id === id)?.name ?? id
  const sceneLink = (id: string) => `/campagnes/${campaignId}/preparer?onglet=graph&scene=${encodeURIComponent(id)}`

  const mapView = !entry || !map ? (
    <p className="text-body text-mute-soft">{t('atlas.empty')}</p>
  ) : (
    <div
      key={entry.map.id}
      className={cn(
        'min-w-0',
        motion === 'in' && 'animate-zoom-in',
        motion === 'out' && 'animate-zoom-out',
      )}
      onAnimationEnd={() => setMotion(null)}
    >
      {scaleOf(entry) === 'world' ? (
        <HexMap
          map={map}
          tileset={tileset}
          party={null}
          marks={marks}
          pins={pinMarks}
          onPin={placing ? undefined : pick}
          onHex={tap}
          className="max-h-[70dvh]"
        />
      ) : (
        <MapCanvas
          scene={{ map, tileset, atlases, tokens: [], backdrop, highlight: marks }}
          onCell={tap}
          pins={pinMarks}
          onPin={placing ? undefined : pick}
          touch={tablet}
          className="max-h-[70dvh]"
        />
      )}
    </div>
  )

  const side = entry && (
    <Panel
      title={entry.map.name}
      actions={
        tablet ? (
          <Btn aria-expanded={panel} onClick={() => setPanel(false)}>
            {t('atlas.fold')}
          </Btn>
        ) : undefined
      }
    >
      <p className="text-caption text-mute-soft">
        {t(`atlas.scale.${scaleOf(entry)}`)} · {t(`atlas.origin.${entry.origin}`)} ·{' '}
        {entry.validated ? t('maps.validated') : t('maps.draft')}
      </p>
      {entry.map.gm_notes && <p className="text-body text-chalk-soft">{entry.map.gm_notes}</p>}
      <PinsSection
        chosen={chosen}
        places={places}
        busy={busy}
        pinning={pinning}
        choice={choice}
        failed={failed}
        maps={maps}
        unplaced={state.atlas.unplaced}
        issues={state.atlas.issues}
        sceneLink={sceneLink}
        placeName={placeName}
        onChoice={setChoice}
        onPin={(place) => {
          setPlacing({ kind: 'place', id: place })
          setFailed(null)
        }}
        onCancel={() => setPlacing(null)}
        onUnpin={(place) => {
          setPicked(null)
          void send({ op: 'set', target: place, field: 'anchor', value: null })
        }}
        onOpenMap={(id) => go(enter(here, id), 'in')}
        onGenerate={(scene) => void generate(scene)}
      />
      <CastSection
        cast={entry.cast}
        castable={state.atlas.castable}
        picked={castPicked}
        placing={placing}
        busy={busy}
        maps={maps}
        sceneLink={sceneLink}
        onPlace={(target) => {
          setPlacing(target)
          setFailed(null)
        }}
        onCancel={() => setPlacing(null)}
        onRemove={(p) => {
          setPicked(null)
          void send(removeEdit(state.atlas.castable, p))
        }}
        onOpenMap={(id) => go(enter(here, id), 'in')}
      />
      <h3 className="type-label text-mute-soft">{t('atlas.inside')}</h3>
      {entry.children.length === 0 ? (
        <p className="text-caption text-mute-soft">{t('atlas.noInside')}</p>
      ) : (
        <ul className="flex flex-col gap-1.5">
          {entry.children.map((id) => (
            <li key={id}>
              <Btn main onClick={() => open(id)}>
                {t('atlas.enter', { name: name(id) })}
              </Btn>
            </li>
          ))}
        </ul>
      )}
      {entry.links.length > 0 && (
        <>
          <h3 className="type-label text-mute-soft">{t('atlas.next')}</h3>
          <ul className="flex flex-col gap-1.5">
            {entry.links.map((l) => (
              <li key={l.exit}>
                <Btn onClick={() => jump(l.to)}>{t('atlas.goTo', { name: name(l.to) })}</Btn>
              </li>
            ))}
          </ul>
        </>
      )}
      {entry.origin === 'campaign' ? (
        <Link
          className="text-caption text-chalk underline underline-offset-4"
          to={`/campagnes/${campaignId}/cartes/${encodeURIComponent(entry.map.id)}`}
        >
          {t('atlas.edit')}
        </Link>
      ) : (
        <Link className="text-caption text-chalk underline underline-offset-4" to={`/campagnes/${campaignId}/cartes`}>
          {t('atlas.copy')}
        </Link>
      )}
    </Panel>
  )

  return (
    <TouchScreen.Provider value={tablet}>
      <main className="surface-table flex min-h-dvh flex-col gap-3 p-4 text-chalk sm:p-5">
        <header className="flex flex-wrap items-center gap-3">
          {back}
          <h1 className="type-title text-[16px]">{t('atlas.title')}</h1>
          <nav aria-label={t('atlas.breadcrumb')} className="flex min-w-0 flex-wrap items-center gap-1.5">
            {crumbs.map((c, i) => (
              <span key={c.map.id} className="flex items-center gap-1.5">
                {i > 0 && <span className="text-mute-soft">›</span>}
                <button
                  type="button"
                  className="pixel-choice py-1 pr-2 pb-1.5 text-caption font-semibold"
                  aria-current={i === crumbs.length - 1 ? 'page' : undefined}
                  onClick={i === crumbs.length - 1 ? undefined : () => go(climb(here, maps, c.map.id), 'out')}
                >
                  <span className="text-mute-soft">{t(`atlas.scale.${scaleOf(c)}`)} · </span>
                  {c.map.name}
                </button>
              </span>
            ))}
          </nav>
          <div className="ml-auto flex gap-1.5">
            <Btn disabled={!canLeave(here, maps)} onClick={out}>
              {t('atlas.out')}
            </Btn>
            <Btn aria-expanded={drawer} onClick={() => setDrawer((d) => !d)}>
              {t('atlas.drawer')}
            </Btn>
            {tablet && !panel && (
              <Btn aria-expanded={panel} onClick={() => setPanel(true)}>
                {t('atlas.unfold')}
              </Btn>
            )}
          </div>
        </header>

        {drawer && (
          <nav aria-label={t('atlas.drawerTitle')} className="surface-slab flex flex-wrap items-center gap-2 p-3">
            <span className="type-label text-mute-soft">{t('atlas.drawerTitle')}</span>
            {DRAWER.map((tab) => (
              <Link
                key={tab}
                className="pixel-key pixel-key-dark pixel-cursor min-h-8 py-1 pr-2.5 pb-2 text-label"
                to={`/campagnes/${campaignId}/preparer?onglet=${tab}`}
              >
                {t(`atlas.place.${tab}`)}
              </Link>
            ))}
          </nav>
        )}

        <div className="flex flex-wrap items-center gap-1.5" role="group" aria-label={t('atlas.storyLayers.label')}>
          <span className="type-label text-mute-soft">{t('atlas.storyLayers.label')}</span>
          {STORY_LAYERS.map((l) => (
            <button
              key={l}
              type="button"
              className="pixel-choice py-1 pr-2 pb-1.5 text-caption font-semibold"
              aria-pressed={lit.has(l)}
              onClick={() =>
                setLit((s) => {
                  const next = new Set(s)
                  if (next.has(l)) next.delete(l)
                  else next.add(l)
                  return next
                })
              }
            >
              {t(`atlas.storyLayers.${l}`)}
            </button>
          ))}
        </div>

        {layers.length > 0 && (
          <div className="flex flex-wrap items-center gap-1.5" role="group" aria-label={t('atlas.layers')}>
            <span className="type-label text-mute-soft">{t('atlas.layers')}</span>
            {layers.map((l) => (
              <button
                key={l.id}
                type="button"
                className="pixel-choice py-1 pr-2 pb-1.5 text-caption font-semibold"
                aria-pressed={!hidden.has(l.id)}
                onClick={() =>
                  setHidden((h) => {
                    const next = new Set(h)
                    if (next.has(l.id)) next.delete(l.id)
                    else next.add(l.id)
                    return next
                  })
                }
              >
                {l.name}
              </button>
            ))}
          </div>
        )}

        <div className={cn('grid min-h-0 flex-1 gap-3', (!tablet || panel) && 'lg:grid-cols-[minmax(0,1fr)_320px]')}>
          <div className="min-w-0 overflow-hidden">{mapView}</div>
          {(!tablet || panel) && side}
        </div>

        <nav aria-label={t('atlas.strip')} className="flex gap-1.5 overflow-x-auto pb-1">
          {state.atlas.maps.map((m) => (
            <button
              key={m.map.id}
              type="button"
              className="pixel-choice flex shrink-0 items-center gap-1.5 py-1.5 pr-2.5 pb-2 text-caption font-semibold"
              aria-current={m.map.id === shownId ? 'page' : undefined}
              onClick={() => jump(m.map.id)}
            >
              <span className="text-mute-soft" aria-hidden="true">
                {'·'.repeat(depth(maps, m.map.id))}
              </span>
              <span className="text-mute-soft">{t(`atlas.scaleShort.${scaleOf(m)}`)}</span>
              {m.map.name}
            </button>
          ))}
        </nav>
      </main>
    </TouchScreen.Provider>
  )
}

/**
 * The side panel's pins (campaign/pin-scenes-on-maps): the pin chosen
 * and its scenes, each opening the scene sheet or its fight map; the pin
 * tool, which anchors a place on the next cell tapped; the scenes that
 * stand nowhere and the pins that stand on nothing.
 */
function PinsSection({
  chosen,
  places,
  busy,
  pinning,
  choice,
  failed,
  maps,
  unplaced,
  issues,
  sceneLink,
  placeName,
  onChoice,
  onPin,
  onCancel,
  onUnpin,
  onOpenMap,
  onGenerate,
}: {
  chosen: Pin | null
  places: Atlas['places']
  busy: boolean
  pinning: string | null
  choice: string
  failed: string | null
  maps: Map<string, AtlasMap>
  unplaced: Atlas['unplaced']
  issues: Atlas['issues']
  sceneLink: (id: string) => string
  placeName: (id: string) => string
  onChoice: (place: string) => void
  onPin: (place: string) => void
  onCancel: () => void
  onUnpin: (place: string) => void
  onOpenMap: (map: string) => void
  onGenerate: (scene: string) => void
}) {
  const { t } = useTranslation()
  const link = 'text-caption text-chalk underline underline-offset-4'
  return (
    <>
      {chosen && (
        <section aria-label={t('atlas.pins.chosen', { name: chosen.name })} className="flex flex-col gap-1.5">
          <h3 className="type-label text-mute-soft">{t('atlas.pins.chosen', { name: chosen.name })}</h3>
          <ul className="flex flex-col gap-1.5">
            {chosen.scenes.map((sc) => (
              <li key={sc.id} className="flex flex-wrap items-center gap-2">
                <Link className={link} to={sceneLink(sc.id)}>
                  {t('atlas.pins.scene', { number: sc.number, title: sc.title })}
                </Link>
                {sc.map && maps.has(sc.map) ? (
                  <Btn onClick={() => onOpenMap(sc.map!)}>{t('atlas.pins.openMap')}</Btn>
                ) : (
                  !sc.map && (
                    <Btn disabled={busy} onClick={() => onGenerate(sc.id)}>
                      {t('atlas.pins.generate')}
                    </Btn>
                  )
                )}
              </li>
            ))}
          </ul>
          <div className="flex flex-wrap gap-1.5">
            <Btn disabled={busy} onClick={() => onPin(chosen.location)}>
              {t('atlas.pins.move')}
            </Btn>
            <Btn disabled={busy} onClick={() => onUnpin(chosen.location)}>
              {t('atlas.pins.remove')}
            </Btn>
          </div>
        </section>
      )}
      <h3 className="type-label text-mute-soft">{t('atlas.pins.tool')}</h3>
      {pinning ? (
        <div className="flex flex-wrap items-center gap-1.5" role="status">
          <p className="text-caption text-chalk-soft">{t('atlas.pins.tapCell', { name: placeName(pinning) })}</p>
          <Btn onClick={onCancel}>{t('atlas.pins.cancel')}</Btn>
        </div>
      ) : places.length === 0 ? (
        <p className="text-caption text-mute-soft">{t('atlas.pins.noPlaces')}</p>
      ) : (
        <div className="flex flex-wrap items-center gap-1.5">
          <select className={field} value={choice} onChange={(e) => onChoice(e.target.value)} aria-label={t('atlas.pins.place')}>
            <option value="">{t('atlas.pins.choose')}</option>
            {places.map((p) => (
              <option key={p.id} value={p.id}>
                {p.anchor ? t('atlas.pins.placed', { name: p.name }) : p.name}
              </option>
            ))}
          </select>
          <Btn main disabled={!choice || busy} onClick={() => onPin(choice)}>
            {t('atlas.pins.pinHere')}
          </Btn>
        </div>
      )}
      {failed && (
        <p role="alert" className="text-caption text-stat-hp">
          {t('atlas.pins.failed', { code: failed })}
        </p>
      )}
      {unplaced.length > 0 && (
        <>
          <h3 className="type-label text-mute-soft">{t('atlas.pins.unplaced')}</h3>
          <ul className="flex flex-col gap-1">
            {unplaced.map((sc) => (
              <li key={sc.id}>
                <Link className={link} to={sceneLink(sc.id)}>
                  {t('atlas.pins.scene', { number: sc.number, title: sc.title })}
                </Link>
              </li>
            ))}
          </ul>
        </>
      )}
      {issues.length > 0 && (
        <ul className="flex flex-col gap-1" aria-label={t('atlas.pins.issues')}>
          {issues.map((i) => (
            <li key={i.path} className="text-caption text-stat-hp">
              {t(`atlas.pins.issue.${i.code}`, { defaultValue: i.detail })}
            </li>
          ))}
        </ul>
      )}
    </>
  )
}
