import { Suspense, lazy, useEffect, useId, useMemo, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Link, Navigate, useNavigate, useParams, useSearchParams } from 'react-router'
import { Btn, Panel, TouchScreen, field } from '@/features/gm/live/ui'
import { useLiveChanges } from '@/features/live/useLiveChanges'
import { MapCanvas } from '@/features/map/MapCanvas'
import { gmMarks, type ThreadMark } from '@/features/map/pins'
import { useImage } from '@/features/map/useImage'
import { useTileset } from '@/features/map/useTileset'
import { HexMap } from '@/features/travel/HexMap'
import { ApiError } from '@/lib/api'
import { archiveCampaign, fetchCampaign, type CampaignDetail } from '@/lib/campaigns'
import type { Cell } from '@/lib/board'
import { fetchAtlas, generateMap, gmBackdropUrl, type Atlas, type AtlasMap, type Pin, type ThreadStop } from '@/lib/maps'
import { applyEdits, decideClaude, setClaudeHand, type ClaudeDecision, type Edit, type Proposal } from '@/lib/prep'
import { decideImage, fetchGmMedia, gmImageUrl, type MediaList } from '@/lib/media'
import { fetchNeeds, type Needs } from '@/lib/elements'
import { buildSceneOnce } from '@/features/hd2d/build'
import { TABLET_QUERY, useMediaQuery } from '@/lib/useMediaQuery'
import { cn } from '@/lib/utils'
import {
  atWidest,
  canLeave,
  climb,
  depth,
  doorAt,
  doorCells,
  enter,
  index,
  leave,
  switchable,
  trail,
  widestRoot,
  withoutLayers,
} from './atlas'
import { ClaudeBanner } from '@/features/gm/prep/ClaudeBanner'
import { usePrepChanges } from '@/features/gm/prep/usePrepChanges'
import { MapsDesk } from '@/features/gm/maps/MapsDesk'
import { MediaDesk } from '@/features/gm/prep/MediaDesk'
import { PREP_TABS, PrepDesk, type PrepTab } from '@/features/gm/prep/PrepDesk'
import { CastSection } from './CastSection'
import { batchOf, proposedMarks } from './claude'
import { ClaudeSection } from './ClaudeSection'
import { STORY_LAYERS, castMarks, placeEdit, readKey, removeEdit, type StoryLayer, type Target } from './cast'
import { Hd2dSection } from './Hd2dSection'
import { PanelTabs, panelTabOf, shownPanelTab, type PanelTab } from './PanelTabs'
import { IdeaSection } from './IdeaSection'
import { ThreadSection } from './ThreadSection'
import { PlayersMap, PlayersSection, usePlayersMap } from './PlayersView'

/** Three.js weighs half a megabyte: only a GM who opens the HD-2D view loads it. */
const Hd2dView = lazy(() => import('@/features/hd2d/Hd2dView').then((m) => ({ default: m.Hd2dView })))

type PageState =
  | { kind: 'loading' }
  | { kind: 'signed-out' }
  | { kind: 'not-found' }
  | { kind: 'error' }
  | { kind: 'ready'; atlas: Atlas }

/** The three scales, as the breadcrumb names them. */
type ScaleName = 'world' | 'place' | 'encounter'
const scaleOf = (m: AtlasMap): ScaleName => m.map.scale ?? 'encounter'

/** The live topics the drawer's tabs are read from. */
const DESK_TOPICS = ['story', 'prep', 'desk']

/** The live topics what the players see is built from. */
const SEEN_TOPICS = ['map', 'world', 'story', 'prep']

/**
 * The drawer (`?onglet=`): what has no place on a map — the scenes and
 * acts with their sheets, the bible, the sheets, the coherence, the
 * validation — then the campaign's images and its own maps.
 */
const DRAWER = [...PREP_TABS, 'medias', 'cartes'] as const
type DrawerTab = (typeof DRAWER)[number]
const isPrep = (tab: DrawerTab): tab is PrepTab => (PREP_TABS as readonly string[]).includes(tab)

const dateFormat = new Intl.DateTimeFormat('fr-FR', { dateStyle: 'long' })

/**
 * `/campagnes/:campaignId` (and `…/atlas`) — entering a campaign opens
 * its Atlas on the widest map (campaign/write-story-by-map-scale): the
 * story is written at each scale of the maps. At the widest, « L'idée »
 * in the side panel holds the campaign's title, universe, idea, hook,
 * tone and themes, then its acts and fronts, the table's settings folded
 * below; a campaign with no map shows « L'idée » alone and the way to
 * its maps. The header carries the campaign's actions (the evening, the
 * table, the rules, the shelf) and its state, as a chip whose sentence
 * shows on hover or tap.
 *
 * campaign/tidy-atlas-layout: the side panel shows one tab at a time
 * (`?panneau=idee|carte|claude`) — « L'idée » at the widest scale only,
 * where it opens first and widens the panel; « La carte » (the map's
 * thread, pins, cast, HD-2D) elsewhere; « Claude » with its waiting
 * batches counted on the tab. A batch arriving lights the count and never
 * changes the tab; a batch tapped on the map or in « L'idée » opens
 * Claude's tab, a pin or piece tapped opens « La carte ». Above the map,
 * as little height as possible: one header row, one row for the
 * breadcrumb and the two view switches, one for every layer.
 *
 * maps/browse-atlas and the shell of
 * gm/prep-on-atlas: every map of the world and the campaign, nested by
 * their exits (region in hexes › place › encounter). The GM enters a
 * place by tapping where it opens on its parent (or from the side
 * panel), and zooming out goes back where they came from. The
 * breadcrumb, the map's own layers, the side panel and the strip of
 * every map at the bottom frame it. On a tablet the map is handled with
 * the fingers and the side panel folds away.
 *
 * It is the one preparation screen (gm/prep-on-atlas): its drawer opens,
 * in place of the map, what has no place on one — scenes and acts with
 * their sheet and the co-GM's workshop, bible, sheets, coherence,
 * validation — then the images and the campaign's own maps (`?onglet=`,
 * `?scene=`). « Vue joueurs » (maps/preview-as-player, `?vue=joueurs`)
 * draws the map as the server projects it for the players, and beside
 * it what their screens say.
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
 * The campaign's thread (campaign/draw-story-thread) joins the pins:
 * one arrow per pair of places an exit leads between, so a branch shows
 * as two. The scenes of the map are listed beside it, for one act or all;
 * a scene tapped lights its arrows and brings its pin into view, a pin
 * tapped marks its scenes in the list. Every revision received redraws
 * it — an exit added in the scene editor appears with no other gesture.
 *
 * Claude works live (platform/propose-edits-by-claude): what it writes
 * with the GM's token waits in batches, drawn dotted on the maps and
 * marked ▶ — pins, NPCs, clues, fights, thread arrows, the cells its map
 * operations touch — until the GM keeps an item, discards one with a
 * word Claude reads back, or keeps everything left. « Claude a la
 * main » lets its writes land directly; the banner then says what it
 * wrote, and takes it back.
 *
 * « HD-2D » (campaign/watch-prep-in-hd2d, `?rendu=hd2d`) turns the map
 * shown into the scene as the table will see it: the grid in 3D, the
 * theme's PixelLab elements, the NPCs' figures, and Claude's proposals
 * floating dotted until kept — rebuilt at every revision, so the GM
 * watches Claude prepare. What it still lacks is listed beside it, and
 * the PixelLab images waiting are kept or discarded there.
 *
 * GM only: a player receives the one map on the table, through the
 * board's projection, never the atlas.
 */
export function AtlasPage() {
  const { t } = useTranslation()
  const navigate = useNavigate()
  const { campaignId = '' } = useParams()
  const [params, setParams] = useSearchParams()
  const [state, setState] = useState<PageState>({ kind: 'loading' })
  /** The campaign itself: « L'idée », its state and its actions; `null` until read. */
  const [campaign, setCampaign] = useState<CampaignDetail | null>(null)
  const [confirmArchive, setConfirmArchive] = useState(false)
  const [shelfBusy, setShelfBusy] = useState(false)
  const [shelfFailed, setShelfFailed] = useState(false)
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
  /** The thread's act (`null`: all), the scene chosen in the list, the cell brought into view. */
  const [act, setAct] = useState<string | null>(null)
  const [scene, setScene] = useState<string | null>(null)
  const [focus, setFocus] = useState<Cell | null>(null)
  /** Claude's batch chosen on the map or in the panel, and the panel's own gestures. */
  const [batch, setBatch] = useState<string | null>(null)
  const [claudeBusy, setClaudeBusy] = useState(false)
  const [claudeFailed, setClaudeFailed] = useState<string | null>(null)
  /** « Vue MJ / Vue joueurs » (maps/preview-as-player), kept in `?vue=joueurs`. */
  const players = params.get('vue') === 'joueurs'
  /** « Plan / HD-2D » (campaign/watch-prep-in-hd2d), kept in `?rendu=hd2d`. */
  const hd2d = params.get('rendu') === 'hd2d'
  /** What the map shown lacks from PixelLab: `map` and `read` (a count) say which reading it is. */
  const [needs, setNeeds] = useState<{ map: string; read: number; needs: Needs } | null>(null)
  const [mediaBusy, setMediaBusy] = useState(false)
  const [mediaFailed, setMediaFailed] = useState<string | null>(null)
  const [seen, setSeen] = useState(0)
  /** The drawer's tab open instead of the map, and the scene chosen in it. */
  const tab = DRAWER.find((x) => x === params.get('onglet')) ?? null
  const deskScene = params.get('scene')
  /** What the drawer hears live: the story and the jobs, then the co-GM's and Claude's batches. */
  const [deskLive, setDeskLive] = useState(0)
  const [batchesLive, setBatchesLive] = useState(0)

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

  // Read again with the atlas: Claude's writes and the GM's decisions move « L'idée » too.
  useEffect(() => {
    let live = true
    fetchCampaign(campaignId).then(
      (c) => {
        if (live) setCampaign(c)
      },
      () => {},
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
    // A PixelLab image brought in or decided moves the `desk` topic.
  }, [campaignId, deskLive])

  // A map added, changed or removed (the GM elsewhere, Claude) moves the
  // GM-only `prep` topic; Claude's batches, the GM-only `proposals`.
  const changes = usePrepChanges(campaignId, state.kind === 'ready' ? state.atlas.revision : null)

  const atlas = state.kind === 'ready' ? state.atlas : null
  const maps = useMemo(() => (atlas ? index(atlas) : new Map<string, AtlasMap>()), [atlas])
  // The map shown: the last walked that still exists, else the widest root.
  const shownId = [...walk].reverse().find((id) => maps.has(id)) ?? (atlas ? widestRoot(atlas, maps) : null)
  const entry = shownId ? (maps.get(shownId) ?? null) : null
  const map = entry ? withoutLayers(entry.map, hidden) : null
  const { tileset, atlases } = useTileset(entry?.map ?? null, media, (id) => gmImageUrl(campaignId, id))
  const backdrop = useImage(entry?.backdrop && entry.map.backdrop?.image ? gmBackdropUrl(campaignId, entry.map.id) : null)
  const playersMap = usePlayersMap(campaignId, shownId, players)
  const flatOnly = entry ? scaleOf(entry) === 'world' : true

  // What the scene lacks from PixelLab, read again at every revision and image.
  useEffect(() => {
    if (!hd2d || !shownId || flatOnly) return
    let live = true
    fetchNeeds(campaignId, shownId).then(
      (n) => {
        if (live) setNeeds((was) => ({ map: shownId, read: (was?.read ?? 0) + 1, needs: n }))
      },
      () => {
        if (live) setNeeds(null)
      },
    )
    return () => {
      live = false
    }
  }, [campaignId, shownId, hd2d, flatOnly, version, deskLive])
  const live = useLiveChanges(
    campaignId,
    (topics) => {
      if (topics.includes('prep') || topics.includes('proposals')) setVersion((v) => v + 1)
      if (topics.some((x) => DESK_TOPICS.includes(x))) setDeskLive((v) => v + 1)
      if (topics.includes('proposals')) setBatchesLive((v) => v + 1)
      // What players see moves with the table (map, world) and the story.
      if (players && topics.some((x) => SEEN_TOPICS.includes(x))) {
        void playersMap.reload()
        setSeen((v) => v + 1)
      }
      changes.onTopics(topics)
    },
    'gm',
    changes.onActivity,
  )

  if (state.kind === 'signed-out') return <Navigate to="/connexion" replace />
  const back = (
    <Link className="text-caption text-mute-soft underline underline-offset-4" to="/">
      {t('gm.campaign.back')}
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
  /** The side panel's tab: « L'idée » is offered at the widest scale only. */
  const widest = !entry || atWidest(state.atlas, entry)
  const panelTab = shownPanelTab(panelTabOf(params.get('panneau')), widest)
  const panelTabs: PanelTab[] = widest ? ['idee', 'carte', 'claude'] : ['carte', 'claude']
  const waiting = (state.atlas.claude ?? []).filter((p) => p.status === 'pending').length

  function go(next: string[], how: 'in' | 'out' | null) {
    setWalk(next)
    setMotion(how)
    setHidden(new Set())
    setPicked(null)
    setPlacing(null)
    setScene(null)
    setFocus(null)
    const last = next[next.length - 1]
    if (last) query({ carte: last, onglet: null, scene: null })
  }
  /** Change some of the address's parameters, keep the others (`null` removes one). */
  function query(patch: Record<string, string | null>) {
    const next = new URLSearchParams(params)
    for (const [key, value] of Object.entries(patch)) {
      if (value === null) next.delete(key)
      else next.set(key, value)
    }
    setParams(next, { replace: true })
  }
  /** Open a tab of the side panel; the choice stays in the address, map after map. */
  function showPanel(next: PanelTab) {
    if (next !== panelTab) query({ panneau: next })
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
  /** The GM's word on one of Claude's batches, then the atlas again. */
  async function decide(p: Proposal, decision: ClaudeDecision) {
    setClaudeBusy(true)
    setClaudeFailed(null)
    try {
      await decideClaude(campaignId, p.id, decision)
      setVersion((v) => v + 1)
    } catch (err) {
      setClaudeFailed(err instanceof ApiError ? err.code : 'action')
    } finally {
      setClaudeBusy(false)
    }
  }
  async function hand(on: boolean) {
    setClaudeBusy(true)
    setClaudeFailed(null)
    try {
      await setClaudeHand(campaignId, on)
      setVersion((v) => v + 1)
    } catch (err) {
      setClaudeFailed(err instanceof ApiError ? err.code : 'action')
    } finally {
      setClaudeBusy(false)
    }
  }
  async function undoClaude() {
    setClaudeBusy(true)
    setClaudeFailed(null)
    try {
      await changes.undo()
      setVersion((v) => v + 1)
      setDeskLive((v) => v + 1)
    } catch (err) {
      setClaudeFailed(err instanceof ApiError ? err.code : 'action')
    } finally {
      setClaudeBusy(false)
    }
  }
  /** Shelve the campaign, or take it off the shelf. */
  async function shelve(archived: boolean) {
    setShelfBusy(true)
    setShelfFailed(false)
    try {
      setCampaign(await archiveCampaign(campaignId, archived))
      setConfirmArchive(false)
    } catch {
      setShelfFailed(true)
    } finally {
      setShelfBusy(false)
    }
  }
  /** A batch's focus: its map, and the cell it is about. */
  const focusOn = (p: Proposal) => {
    setBatch(p.id)
    if (!p.focus) return
    if (maps.has(p.focus.map) && p.focus.map !== shownId) go(enter(here, p.focus.map), 'in')
    setFocus(p.focus.at ?? null)
  }
  /** A tap on a mark: Claude's opens its batch, the campaign's is picked. */
  const onMark = (key: string) => {
    const of = batchOf(key)
    if (!of) {
      pick(key)
      return
    }
    const next = batch === of ? null : of
    setBatch(next)
    if (next) showPanel('claude')
  }
  /** A batch chosen from the map or « L'idée »: the GM reads it in Claude's tab. */
  const pickBatch = (id: string) => {
    setBatch(id)
    showPanel('claude')
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
  const pick = (place: string) => {
    setScene(null)
    setPicked((p) => (p === place ? null : place))
    // What a pin or a piece says (its scenes, its fight) stands in « La carte ».
    if (picked !== place) showPanel('carte')
  }
  const choose = (stop: ThreadStop) => {
    setScene(stop.scene)
    setPicked(stop.place)
    setFocus(pins.find((p) => p.location === stop.place)?.at ?? null)
  }

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
  const claudeMarks = proposedMarks(
    entry?.proposed ?? [],
    lit,
    {
      pin: (p) => t('atlas.claude.mark', { title: pinTitle(p) }),
      npc: (name, scene) =>
        t('atlas.claude.mark', { title: scene ? t('atlas.cast.npcFor', { name, scene: sceneName(scene) }) : name }),
      clue: (text) => t('atlas.claude.mark', { title: t('atlas.cast.clue', { text }) }),
      fight: (number, title) => t('atlas.claude.mark', { title: t('atlas.cast.fight', { number, title }) }),
      segment: (from, to) =>
        t('atlas.claude.mark', { title: t('atlas.thread.segment', { from: placeName(from), to: placeName(to), scenes: '' }) }),
    },
    batch,
  )
  const actsHere = state.atlas.acts.filter((a) => entry?.thread.stops.some((s) => s.act === a.id))
  // An act chosen on another map, or whose scenes left this one, falls back to every act.
  const shownAct = act !== null && actsHere.some((a) => a.id === act) ? act : null
  const inAct = (a: string) => shownAct === null || a === shownAct
  const stops = (entry?.thread.stops ?? []).filter((s) => inAct(s.act))
  const pickedPlace = pins.some((p) => p.location === picked) ? picked : null
  const number = (id: string) => state.atlas.castable.scenes.find((x) => x.id === id)?.number ?? '?'
  const threadMarks: ThreadMark[] = !entry || !lit.has('story')
    ? []
    : entry.thread.segments
        .map((sg) => ({ ...sg, steps: sg.steps.filter((st) => inAct(st.act)) }))
        .filter((sg) => sg.steps.length > 0)
        .map((sg) => {
          const on = scene
            ? sg.steps.some((st) => st.from === scene || st.to === scene)
            : pickedPlace !== null && (sg.from.place === pickedPlace || sg.to.place === pickedPlace)
          return {
            key: `${sg.from.place}>${sg.to.place}`,
            from: sg.from.at,
            to: sg.to.at,
            title: t('atlas.thread.segment', {
              from: placeName(sg.from.place),
              to: placeName(sg.to.place),
              scenes: sg.steps.map((st) => `${number(st.from)} → ${number(st.to)}`).join(', '),
            }),
            lit: on,
            faint: (scene !== null || pickedPlace !== null) && !on,
          }
        })
  /** The scene an issue's path is about (`nodes[3].exits[0]`), by its number and title. */
  const sceneAt = (path: string) => {
    const i = /^nodes\[(\d+)\]/.exec(path)
    const sc = i ? state.atlas.castable.scenes[Number(i[1])] : undefined
    return sc ? t('atlas.pins.scene', { number: sc.number, title: sc.title }) : ''
  }
  const sceneLink = (id: string) => {
    const at = new URLSearchParams({ ...(shownId ? { carte: shownId } : {}), onglet: 'graph', scene: id })
    return `/campagnes/${campaignId}/atlas?${at.toString()}`
  }

  /** Switch « Vue MJ / Vue joueurs »: the map's own id stays in the address. */
  const switchView = (on: boolean) => {
    query({ carte: shownId, vue: on ? 'joueurs' : null, onglet: null, scene: null })
    setPicked(null)
    setPlacing(null)
  }
  const openTab = (next: DrawerTab | null) => query({ carte: shownId, onglet: next, scene: null })
  const switchRender = (on: boolean) => query({ carte: shownId, rendu: on ? 'hd2d' : null })
  /** The GM's word on a PixelLab image, from beside the scene. */
  async function decideMedia(asset: string, keep: boolean) {
    setMediaBusy(true)
    setMediaFailed(null)
    try {
      await decideImage(campaignId, asset, keep)
      setDeskLive((v) => v + 1)
    } catch (err) {
      setMediaFailed(err instanceof ApiError ? err.code : 'action')
    } finally {
      setMediaBusy(false)
    }
  }
  // Another map's needs never dress this one, even for the instant its own are read.
  const ownNeeds = needs && needs.map === shownId ? needs : null
  const marks3d = [...pinMarks, ...claudeMarks.pins]
  // Built again only when what it shows changed, not on every render of the page.
  const sceneKey = [
    campaignId,
    shownId,
    state.atlas.revision,
    [...hidden].join(','),
    ownNeeds?.read ?? 0,
    (media?.assets ?? []).map((a) => `${a.id}:${a.status ?? ''}`).join(','),
    marks3d.map((m) => `${m.key}@${m.at.join(',')}:${m.picked ? 1 : 0}${m.faint ? 1 : 0}`).join(';'),
    claudeMarks.cells.map((c) => c.join(',')).join(';'),
    tileset?.id ?? '',
  ].join('|')
  const scene3d =
    hd2d && !players && entry && map && !flatOnly
      ? buildSceneOnce(sceneKey, {
          map,
          tileset,
          needs: ownNeeds?.needs ?? null,
          assets: media?.assets ?? [],
          marks: marks3d,
          proposedCells: claudeMarks.cells,
          imageUrl: (id) => gmImageUrl(campaignId, id),
          npcOf: (key) => {
            const k = readKey(key)
            return k?.kind === 'npc' ? k.id : null
          },
          batchOf,
        })
      : null
  const pickedBatch = (state.atlas.claude ?? []).find((p) => p.id === batch) ?? null

  const desk =
    tab === null ? null : isPrep(tab) ? (
      <PrepDesk
        campaignId={campaignId}
        tab={tab}
        scene={deskScene}
        live={deskLive}
        proposals={batchesLive}
        fresh={changes.fresh}
        onScene={(id) => query({ scene: id })}
        onMap={() => openTab(null)}
      />
    ) : tab === 'medias' ? (
      <MediaDesk campaignId={campaignId} live={deskLive} />
    ) : (
      <MapsDesk campaignId={campaignId} live={deskLive} />
    )

  const mapView = !entry || !map ? (
    <p className="text-body text-mute-soft">{t('atlas.empty')}</p>
  ) : scene3d ? (
    <div key={`hd2d-${entry.map.id}`} className="flex min-w-0 flex-col gap-2">
      {pickedBatch && (
        <div role="group" aria-label={t('atlas.hd2d.claude', { title: pickedBatch.prompt || t('atlas.claude.untitled') })} className="surface-slab flex flex-wrap items-center gap-2 p-2">
          <span className="text-caption text-chalk">{t('atlas.hd2d.claude', { title: pickedBatch.prompt || t('atlas.claude.untitled') })}</span>
          <Btn main disabled={claudeBusy} onClick={() => void decide(pickedBatch, { all: 'keep' })}>
            {t('atlas.hd2d.keepAll')}
          </Btn>
          <Btn disabled={claudeBusy} onClick={() => void decide(pickedBatch, { all: 'discard' })}>
            {t('atlas.hd2d.discardAll')}
          </Btn>
          {tablet && !panel && (
            <Btn
              onClick={() => {
                setPanel(true)
                showPanel('claude')
              }}
            >
              {t('atlas.hd2d.details')}
            </Btn>
          )}
        </div>
      )}
      <Suspense fallback={<p role="status" className="text-body text-mute-soft">{t('prep.loading')}</p>}>
        <Hd2dView scene={scene3d} onMark={placing ? undefined : onMark} className="max-h-[70dvh]" />
      </Suspense>
    </div>
  ) : players ? (
    <PlayersMap state={playersMap.state} tileset={tileset} atlases={atlases} backdrop={backdrop} touch={tablet} />
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
          marks={[...marks, ...claudeMarks.cells]}
          pins={[...pinMarks, ...claudeMarks.pins]}
          thread={[...threadMarks, ...claudeMarks.thread]}
          onPin={placing ? undefined : onMark}
          onHex={tap}
          className="max-h-[70dvh]"
        />
      ) : (
        <MapCanvas
          scene={{ map, tileset, atlases, tokens: [], backdrop, highlight: [...marks, ...claudeMarks.cells] }}
          onCell={tap}
          pins={[...pinMarks, ...claudeMarks.pins]}
          thread={[...threadMarks, ...claudeMarks.thread]}
          focus={focus}
          onPin={placing ? undefined : onMark}
          touch={tablet}
          className="max-h-[70dvh]"
        />
      )}
    </div>
  )

  const claudeSection = (
    <ClaudeSection
      batches={state.atlas.claude ?? []}
      picked={batch}
      hand={state.atlas.claudeHand ?? false}
      busy={claudeBusy}
      failed={claudeFailed}
      onPick={setBatch}
      onFocus={focusOn}
      onDecide={(p, d) => void decide(p, d)}
      onHand={(on) => void hand(on)}
    />
  )
  // The widest scale carries the campaign's idea; with no map at all, it stands alone.
  const idea =
    campaign && !players && widest ? (
      <IdeaSection
        key={campaign.id}
        campaignId={campaignId}
        campaign={campaign}
        fresh={changes.fresh}
        claude={{
          batches: state.atlas.claude ?? [],
          busy: claudeBusy,
          onPick: pickBatch,
          onDecide: (p, d) => void decide(p, d),
        }}
        onSaved={(c) => {
          setCampaign(c)
          setVersion((v) => v + 1)
        }}
      />
    ) : null
  const base = `/campagnes/${encodeURIComponent(campaignId)}`

  const side = entry && (
    <Panel title={entry.map.name}>
      <p className="text-caption text-mute-soft">
        {t(`atlas.scale.${scaleOf(entry)}`)} · {t(`atlas.origin.${entry.origin}`)} ·{' '}
        {entry.validated ? t('maps.validated') : t('maps.draft')}
      </p>
      {entry.map.gm_notes && <p className="text-body text-chalk-soft">{entry.map.gm_notes}</p>}
      {scene3d && <Hd2dSection scene={scene3d} busy={mediaBusy} failed={mediaFailed} onDecide={(a, k) => void decideMedia(a, k)} />}
      {hd2d && flatOnly && <p className="text-caption text-mute-soft">{t('atlas.hd2d.world')}</p>}
      <ThreadSection
        stops={stops}
        acts={actsHere}
        act={shownAct}
        scene={scene}
        place={pickedPlace}
        sceneTitle={sceneName}
        onAct={(a) => {
          setAct(a)
          setScene(null)
        }}
        onScene={choose}
      />
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
        sceneAt={sceneAt}
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
        <button type="button" className="self-start text-caption text-chalk underline underline-offset-4" onClick={() => openTab('cartes')}>
          {t('atlas.copy')}
        </button>
      )}
    </Panel>
  )

  return (
    <TouchScreen.Provider value={tablet}>
      <main className="surface-table flex min-h-dvh flex-col gap-3 p-4 text-chalk sm:p-5">
        <header className="flex flex-wrap items-center gap-x-3 gap-y-2">
          {back}
          <h1 className="type-title text-[16px]">{campaign?.story.title ?? t('atlas.title')}</h1>
          {campaign && (
            <StatusChip
              label={campaign.archivedAt ? t('gm.campaigns.status.archived') : t('gm.campaigns.status.prep')}
              hint={
                campaign.archivedAt
                  ? t('gm.campaign.bannerArchived', { date: dateFormat.format(new Date(campaign.archivedAt)) })
                  : t('gm.campaign.bannerPrep')
              }
            />
          )}
          <div className="ml-auto flex flex-wrap gap-1.5" role="group" aria-label={t('gm.campaign.actions')}>
            <Btn main onClick={() => navigate(`${base}/soiree`)}>
              {t('gm.campaign.evening')}
            </Btn>
            <Btn onClick={() => navigate(`${base}/table`)}>{t('gm.campaign.table')}</Btn>
            <Btn onClick={() => navigate(`${base}/regles`)}>{t('gm.campaign.rules')}</Btn>
            {campaign &&
              (campaign.archivedAt ? (
                <Btn disabled={shelfBusy} onClick={() => void shelve(false)}>
                  {t('gm.campaign.unarchive')}
                </Btn>
              ) : (
                <Btn
                  disabled={shelfBusy}
                  onClick={() => (confirmArchive ? void shelve(true) : setConfirmArchive(true))}
                >
                  {confirmArchive ? t('gm.campaign.confirmArchive') : t('gm.campaign.archive')}
                </Btn>
              ))}
          </div>
        </header>

        {shelfFailed && (
          <p role="alert" className="text-caption text-stat-atk">
            {t('gm.campaign.actionError')}
          </p>
        )}

        <div className="flex flex-wrap items-center gap-x-3 gap-y-2">
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
          <div className="ml-auto flex flex-wrap items-center gap-1.5">
            <Segmented
              label={t('atlas.players.switch')}
              options={[
                { label: t('atlas.players.gm'), on: !players, onClick: () => switchView(false) },
                { label: t('atlas.players.players'), on: players, onClick: () => switchView(true) },
              ]}
            />
            {!players && (
              <Segmented
                label={t('atlas.hd2d.switch')}
                options={[
                  { label: t('atlas.hd2d.plan'), on: !hd2d, onClick: () => switchRender(false) },
                  { label: t('atlas.hd2d.hd2d'), on: hd2d, onClick: () => switchRender(true) },
                ]}
              />
            )}
            <Btn disabled={!canLeave(here, maps)} onClick={out}>
              {t('atlas.out')}
            </Btn>
            <Btn aria-expanded={drawer || tab !== null} onClick={() => setDrawer((d) => !d)}>
              {t('atlas.drawer')}
            </Btn>
            {tablet && !panel && (
              <Btn aria-expanded={panel} onClick={() => setPanel(true)}>
                {t('atlas.unfold')}
              </Btn>
            )}
          </div>
        </div>

        <ClaudeBanner
          changes={changes}
          busy={claudeBusy}
          sceneTitle={(id) => state.atlas.castable.scenes.find((x) => x.id === id)?.title}
          onUndo={() => void undoClaude()}
        />

        {(drawer || tab !== null) && (
          <nav aria-label={t('atlas.drawerTitle')} className="surface-slab flex flex-wrap items-center gap-2 p-3">
            <span className="type-label text-mute-soft">{t('atlas.drawerTitle')}</span>
            {DRAWER.map((x) => (
              <button
                key={x}
                type="button"
                className="pixel-tab py-1.5 pb-2"
                aria-current={tab === x ? 'page' : undefined}
                onClick={() => openTab(x)}
              >
                {t(`atlas.place.${x}`)}
              </button>
            ))}
            {tab !== null && (
              <Btn className="ml-auto" onClick={() => openTab(null)}>
                {t('atlas.backToMap')}
              </Btn>
            )}
          </nav>
        )}

        {desk}

        {!players && tab === null && (
          // The story's layers and the map's own share one row.
          <div className="flex flex-wrap items-center gap-x-4 gap-y-1.5">
            <LayerGroup
              label={t('atlas.storyLayers.label')}
              layers={STORY_LAYERS.map((l) => ({ id: l, name: t(`atlas.storyLayers.${l}`), on: lit.has(l) }))}
              onToggle={(id) =>
                setLit((s) => {
                  const l = STORY_LAYERS.find((x) => x === id)
                  if (!l) return s
                  const next = new Set(s)
                  if (next.has(l)) next.delete(l)
                  else next.add(l)
                  return next
                })
              }
            />
            {layers.length > 0 && (
              <LayerGroup
                label={t('atlas.layers')}
                layers={layers.map((l) => ({ id: l.id, name: l.name, on: !hidden.has(l.id) }))}
                onToggle={(id) =>
                  setHidden((h) => {
                    const next = new Set(h)
                    if (next.has(id)) next.delete(id)
                    else next.add(id)
                    return next
                  })
                }
              />
            )}
          </div>
        )}

        {tab === null && !entry && (
          // No map yet: « L'idée » alone, and the way to the campaign's maps.
          <div className="grid gap-3 lg:grid-cols-[minmax(0,640px)_320px]">
            {idea ?? <p className="text-body text-mute-soft">{t('atlas.empty')}</p>}
            <div className="flex flex-col gap-3">
              <Panel title={t('gm.idea.noMap')}>
                <p className="text-caption text-mute-soft">{t('gm.idea.noMapSub')}</p>
                <Btn main className="self-start" onClick={() => openTab('cartes')}>
                  {t('gm.idea.toMaps')}
                </Btn>
              </Panel>
              <div className="surface-slab p-3.5">{claudeSection}</div>
            </div>
          </div>
        )}

        {tab === null && entry && (
          <div
            className={cn(
              'grid min-h-0 flex-1 gap-3',
              (!tablet || panel) &&
                (!players && panelTab === 'idee'
                  ? 'lg:grid-cols-[minmax(0,1fr)_480px]'
                  : 'lg:grid-cols-[minmax(0,1fr)_320px]'),
            )}
          >
            <div className="min-w-0 overflow-hidden">{mapView}</div>
            {(!tablet || panel) &&
              (players ? (
                <PlayersSection campaignId={campaignId} version={seen} live={live} />
              ) : (
                <PanelTabs
                  tabs={panelTabs}
                  current={panelTab}
                  waiting={waiting}
                  onTab={showPanel}
                  aside={
                    tablet ? (
                      <Btn aria-expanded={panel} onClick={() => setPanel(false)}>
                        {t('atlas.fold')}
                      </Btn>
                    ) : undefined
                  }
                >
                  {panelTab === 'idee' ? (
                    (idea ?? (
                      <p role="status" className="text-body text-mute-soft">
                        {t('prep.loading')}
                      </p>
                    ))
                  ) : panelTab === 'claude' ? (
                    <div className="surface-slab p-3.5">{claudeSection}</div>
                  ) : (
                    side
                  )}
                </PanelTabs>
              ))}
          </div>
        )}

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
  sceneAt,
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
  sceneAt: (path: string) => string
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
              {t(`atlas.pins.issue.${i.code}`, { defaultValue: i.detail, scene: sceneAt(i.path) })}
            </li>
          ))}
        </ul>
      )}
    </>
  )
}

/**
 * The campaign's state in the header (campaign/tidy-atlas-layout): a chip
 * whose sentence — what the state means for the table — shows on hover,
 * on keyboard focus, or on a tap for a finger, and is its description for
 * a screen reader.
 */
function StatusChip({ label, hint }: { label: string; hint: string }) {
  const [open, setOpen] = useState(false)
  const id = useId()
  return (
    <span className="group relative">
      <button
        type="button"
        className="pixel-choice py-0.5 pr-2 pb-1 text-[11px] font-semibold text-mute-soft"
        aria-expanded={open}
        aria-describedby={id}
        onClick={() => setOpen((o) => !o)}
        onFocus={(e) => {
          // Keyboard focus shows the sentence; a tap's focus leaves it to the tap.
          if (focusVisible(e.currentTarget)) setOpen(true)
        }}
        onBlur={() => setOpen(false)}
        onKeyDown={(e) => {
          if (e.key === 'Escape') setOpen(false)
        }}
      >
        {label}
      </button>
      <span
        id={id}
        role="tooltip"
        className={cn(
          'surface-slab absolute top-full left-0 z-30 mt-1.5 w-72 p-2.5 text-caption text-chalk-soft',
          open ? 'block' : 'hidden group-hover:block',
        )}
      >
        {hint}
      </span>
    </span>
  )
}

/** Whether the element's focus came from the keyboard (an engine without `:focus-visible` says no). */
function focusVisible(el: Element): boolean {
  try {
    return el.matches(':focus-visible')
  } catch {
    return false
  }
}

/** Exclusive choices drawn as one compact strip of keys: « Vue MJ / Vue joueurs », « Plan / HD-2D ». */
function Segmented({ label, options }: { label: string; options: { label: string; on: boolean; onClick: () => void }[] }) {
  return (
    <div className="pixel-well flex gap-0.5 p-0.5" role="group" aria-label={label}>
      {options.map((o) => (
        <button
          key={o.label}
          type="button"
          className="pixel-choice py-0.5 pr-2 pb-1 text-caption font-semibold"
          aria-pressed={o.on}
          onClick={o.onClick}
        >
          {o.label}
        </button>
      ))}
    </div>
  )
}

/** One family of layers (the story's, the map's own), lit or dimmed one by one. */
function LayerGroup({
  label,
  layers,
  onToggle,
}: {
  label: string
  layers: { id: string; name: string; on: boolean }[]
  onToggle: (id: string) => void
}) {
  return (
    <div className="flex flex-wrap items-center gap-1.5" role="group" aria-label={label}>
      <span className="type-label text-mute-soft">{label}</span>
      {layers.map((l) => (
        <button
          key={l.id}
          type="button"
          className="pixel-choice py-0.5 pr-2 pb-1 text-caption font-semibold"
          aria-pressed={l.on}
          onClick={() => onToggle(l.id)}
        >
          {l.name}
        </button>
      ))}
    </div>
  )
}
