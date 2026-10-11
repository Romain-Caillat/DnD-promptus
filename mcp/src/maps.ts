/**
 * The map tools (platform/edit-maps-by-claude). Claude does not see
 * pixels: it reads the atlas and a map as text, asks the engine on
 * names (path and cost, line of sight, what the players see), and
 * places pins, NPCs, clues and fights « near » something or « along » a
 * path — the server picks the cell (free, dry, reachable, never in a
 * wall or in the water) and says where. Every write is proposed to the
 * GM like apply_story_edits, unless the GM gave Claude the hand.
 */

import type { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js'
import { z } from 'zod'
import { type Promptus, PromptusError } from './promptus'
import { WHY, campaignId, guarded, type Issue, issueReport, type ProposalView, text } from './tools'

type Json = Record<string, unknown>
type Cell = [number, number]

interface PinScene {
  id: string
  title: string
}

interface AtlasMap {
  map: { id: string; name: string; scale: string; grid: { rows: string[] } }
  origin: 'world' | 'campaign'
  parent: { map: string; exit: string } | null
  children: string[]
  validated: boolean
  pins: { location: string; name: string; at: Cell; scenes: PinScene[] }[]
  cast: {
    npcs: { npc: string; at: Cell; scene?: string }[]
    clues: { clue: string; at: Cell }[]
    fights: { scene: string; at: Cell }[]
  }
  proposed: { proposal: string }[]
}

/** One map an act's scenes cross, or the map to create for a place (campaign/order-maps-by-story). */
interface FriezeStop {
  map?: string
  place?: string
  name: string
  scenes: PinScene[]
}

interface Frieze {
  acts: { act: string; title: string; evening: number; slots: { stops: FriezeStop[] }[] }[]
  outside: string[]
}

interface Atlas {
  revision: number
  roots: string[]
  maps: AtlasMap[]
  places: { id: string; name: string; parent?: string | null; anchor?: { map: string; at: Cell } | null }[]
  unplaced: PinScene[]
  issues: Issue[]
  claude: unknown[]
  claudeHand: boolean
  frieze: Frieze
}

interface Mark {
  symbol: string
  kind: 'pin' | 'npc' | 'clue' | 'fight' | 'exit' | 'door' | 'prop' | 'object' | 'start' | 'label'
  id: string
  name: string
  cells: Cell[]
  layer?: 'all' | 'gm' | 'players'
  note?: string
}

interface Terrain {
  glyph: string
  terrain: string
  wall: boolean
  void: boolean
  water: 'none' | 'shallow' | 'deep'
  difficult: boolean
  elevation: number
}

export interface Sketch {
  map: string
  name: string
  scale: 'world' | 'place' | 'encounter'
  width: number
  height: number
  zone: { x: number; y: number; w: number; h: number }
  cut: boolean
  rows: string[]
  terrain: Terrain[]
  legend: Mark[]
}

interface Placed {
  what: 'scene' | 'place' | 'npc' | 'clue' | 'fight'
  id: string
  name: string
  at: Cell
  near: string
  steps: number
  terrain: string
  created?: string
  with?: string[]
}

type PlaceResult =
  | { status: 'applied'; changes: unknown; campaign: { issues: Issue[] }; placed: Placed[] }
  | { status: 'proposed'; why: string; proposal: ProposalView; issues: Issue[]; placed: Placed[] }

const SCALE: Record<Sketch['scale'], string> = {
  world: 'région (hexagones)',
  place: 'lieu',
  encounter: 'rencontre',
}

const KIND: Record<Mark['kind'], string> = {
  pin: 'Épingle',
  npc: 'PNJ',
  clue: 'Indice',
  fight: 'Combat',
  exit: 'Sortie',
  door: 'Porte',
  prop: 'Décor',
  object: 'Objet',
  start: 'Départ',
  label: 'Étiquette',
}

const WHAT: Record<Placed['what'], string> = {
  scene: 'Scène',
  place: 'Lieu',
  npc: 'PNJ',
  clue: 'Indice',
  fight: 'Combat',
}

const at = (c: Cell) => `[${c[0]}, ${c[1]}]`

function cells(list: Cell[], max = 6): string {
  const shown = list.slice(0, max).map(at).join(' ')
  return list.length > max ? `${shown} … (+${list.length - max})` : shown
}

/** A stop of the frieze: the map and its scenes, or the place whose map is still to create. */
function friezeStop(s: FriezeStop): Json {
  const scènes = s.scenes.map((x) => x.id)
  return s.map ? { carte: s.map, nom: s.name, scènes } : { carte_à_créer_pour: s.place, nom: s.name, scènes }
}

/** The atlas as Claude needs it: the tree, what stands on each map, the places, what is not placed, the story's order. */
export function atlasReport(a: Atlas): Json {
  return {
    revision: a.revision,
    claude_a_la_main: a.claudeHand,
    lots_en_attente: a.claude.length,
    racines: a.roots,
    cartes: a.maps.map((m) => ({
      id: m.map.id,
      nom: m.map.name,
      échelle: SCALE[m.map.scale as Sketch['scale']] ?? m.map.scale,
      taille: `${m.map.grid.rows[0]?.length ?? 0} × ${m.map.grid.rows.length}`,
      origine: m.origin === 'world' ? 'monde (lecture, épingles et PNJ seulement)' : 'campagne',
      ...(m.parent ? { dans: `${m.parent.map} (sortie ${m.parent.exit})` } : {}),
      ...(m.children.length ? { contient: m.children } : {}),
      ...(m.origin === 'campaign' && !m.validated ? { validée: false } : {}),
      épingles: m.pins.map((p) => ({ lieu: p.location, nom: p.name, case: at(p.at), scènes: p.scenes.map((s) => s.id) })),
      pnj: m.cast.npcs.map((n) => ({ pnj: n.npc, case: at(n.at), ...(n.scene ? { scène: n.scene } : {}) })),
      indices: m.cast.clues.map((c) => ({ indice: c.clue, case: at(c.at) })),
      combats: m.cast.fights.map((f) => ({ scène: f.scene, case: at(f.at) })),
      ...(m.proposed.length ? { propositions_en_attente: m.proposed.length } : {}),
    })),
    lieux: a.places.map((p) => ({
      id: p.id,
      nom: p.name,
      ...(p.parent ? { dans: p.parent } : {}),
      ...(p.anchor ? { épingle: `${p.anchor.map} ${at(p.anchor.at)}` } : {}),
    })),
    scènes_sans_épingle: a.unplaced.map((s) => `${s.id} (${s.title})`),
    frise: a.frieze.acts.map((act) => ({
      soir: act.evening,
      acte: act.act,
      titre: act.title,
      cases: act.slots.map((slot) => slot.stops.map(friezeStop)),
    })),
    hors_de_l_histoire: a.frieze.outside,
    alertes: issueReport(a.issues),
  }
}

function terrainLine(t: Terrain): string {
  const traits = [
    t.wall && 'mur',
    t.void && 'vide',
    t.water === 'shallow' && 'eau peu profonde',
    t.water === 'deep' && 'eau profonde',
    t.difficult && 'difficile',
    t.elevation !== 0 && `hauteur ${t.elevation}`,
  ].filter(Boolean)
  return `  ${t.glyph}  ${t.terrain}${traits.length ? ` (${traits.join(', ')})` : ''}`
}

function markLine(m: Mark): string {
  const name = m.id && m.name ? `${m.id} (${m.name})` : m.id || m.name
  const note = (() => {
    if (!m.note) return ''
    if (m.kind === 'pin') return ` · scènes ${m.note}`
    if (m.kind === 'npc' || m.kind === 'clue') return ` · scène ${m.note}`
    if (m.kind === 'exit') return ` · vers ${m.note}`
    return ` · ${m.note}`
  })()
  const layer = m.layer === 'gm' ? ' · MJ seul' : m.layer === 'players' ? ' · illusion vue des joueurs' : ''
  return `  ${m.symbol}  ${KIND[m.kind]} ${name} · ${cells(m.cells)}${note}${layer}`
}

/** A map as text: its grid with numbered axes, what each glyph is, what each symbol stands for. */
export function sketchText(s: Sketch): string {
  const z = s.zone
  const head = [
    `Carte « ${s.name} » (${s.map}) · ${SCALE[s.scale]} · ${s.width} × ${s.height} cases`,
    s.cut
      ? `Zone x ${z.x}–${z.x + z.w - 1}, y ${z.y}–${z.y + z.h - 1} : la carte est plus grande, demande une autre zone pour le reste.`
      : '',
    s.scale === 'world'
      ? 'Hexagones : chaque case prend deux caractères, les rangées impaires sont décalées d\'un demi-hexagone vers la droite.'
      : 'Les deux premières lignes donnent x (dizaines, unités), chaque ligne commence par son y. Une case se nomme « x,y ».',
  ].filter(Boolean)
  return [
    ...head,
    '',
    ...s.rows,
    '',
    'Sol :',
    ...s.terrain.map(terrainLine),
    ...(s.legend.length ? ['Éléments posés :', ...s.legend.map(markLine)] : ['Rien de posé dans cette zone.']),
  ].join('\n')
}

function placedLine(p: Placed): string {
  const extra = [
    p.created ? `lieu créé : ${p.created}` : '',
    p.with?.length ? `déplace aussi ${p.with.join(', ')}` : '',
  ].filter(Boolean)
  const where = p.steps === 0 ? `sur « ${p.near} »` : `à ${p.steps} pas de « ${p.near} »`
  return `${WHAT[p.what]} ${p.name} (${p.id}) → ${at(p.at)}, ${p.terrain}, ${where}${extra.length ? ` (${extra.join(' ; ')})` : ''}`
}

/** What a placement request did, as Claude reads it. */
export function placeReport(r: PlaceResult): Json {
  const placed = r.placed.map(placedLine)
  if (r.status === 'proposed') {
    return {
      status: 'proposé',
      why: WHY[r.why] ?? r.why,
      proposal: { id: r.proposal.id, title: r.proposal.prompt, items: r.proposal.items.length },
      placed,
      issues: issueReport(r.issues),
      next: 'En attente du MJ dans Promptus (dessiné en pointillés dans l\'Atlas). list_proposals dira ce qu\'il garde.',
    }
  }
  return { status: 'appliqué', placed, changes: r.changes, issues: issueReport(r.campaign.issues) }
}

const mapId = z.string().min(1).describe("Identifiant de la carte (donné par get_atlas), par ex. port-louis-rues.")
const reference = (what: string) =>
  z
    .string()
    .min(1)
    .describe(
      `${what} : un élément de la carte par son identifiant ou son nom (étiquette, décor, porte, sortie, départ, ` +
        'lieu épinglé, scène, PNJ posé), ou une case « x,y ».',
    )

const PLACE_HELP = `Pose sur une carte, par des gestes qui ont du sens, sans coordonnées : épingler une scène ou un lieu, \
poser un PNJ, un indice ou le combat d'une scène « près de » quelque chose (near) ou « sur le chemin » entre deux \
références (along : plusieurs placements sur le même chemin s'y répartissent dans l'ordre). Le serveur choisit la case : \
la plus proche en marchant depuis la référence, jamais dans un mur ni dans l'eau, libre (rien d'autre n'y est posé, ni \
proposé dans un autre lot), et atteignable depuis l'entrée de la carte ; la réponse dit où chaque chose est tombée. \
Épingler une scène épingle son lieu (les autres scènes de ce lieu suivent) ; une scène sans lieu, ou dont le lieu est \
épinglé sur une autre carte, reçoit un lieu à elle (dans l'ancien). Un PNJ peut avoir une position par scène (scene). \
Tout ou rien. Comme apply_story_edits, c'est proposé au MJ (title, proposal_id), sauf si le MJ t'a donné la main. \
Lis d'abord la carte (get_map_text) pour nommer ses éléments.`

const OPS_HELP = `Redessine une carte de la campagne par opérations (pas une carte du monde : copie-la d'abord dans Promptus). \
La révision lue est envoyée avec ; si la carte a changé au même endroit entre-temps, l'outil relit et réessaie une fois. \
Proposé au MJ comme le reste (title, proposal_id), sauf si le MJ t'a donné la main. Pour poser un PNJ, un indice, une \
épingle ou un combat, préfère place_on_map. Les opérations, avec les cases en [x, y] (lues avec get_map_text) :
- { "op": "paint", "cells": [[x, y], …], "kind": { "terrain": "pavés" } } — kind peut porter "wall": true, \
"water": "shallow"|"deep", "difficult": true, "void": true, "elevation": n ;
- { "op": "place", "at": [x, y], "thing": { "kind": "prop", "prop": "tonneaux", "label": "Tonneaux" } } — thing parmi \
prop (prop, label), object (label), light, start (side: party|foes|neutral, entity?) ;
- { "op": "door", "at": [x, y] } (sur un mur) ; { "op": "move", "id": "…", "to": [x, y] } ; { "op": "remove", "id": "…" } ; \
{ "op": "erase", "at": [x, y] } ; { "op": "set", "name"?: "…", "notes"?: "…" }.`

interface MapRead {
  revision: number
}

type OpsResult =
  | ({ status: 'applied' } & MapRead)
  | (MapRead & { status?: undefined })
  | { status: 'proposed'; why: string; proposal: ProposalView; issues: Issue[] }

export function registerMapTools(server: McpServer, promptus: () => Promptus) {
  const read = { readOnlyHint: true, openWorldHint: false }
  const write = { readOnlyHint: false, destructiveHint: false, idempotentHint: false, openWorldHint: false }
  const path = (campaign: string, map?: string) =>
    `/campaigns/${encodeURIComponent(campaign)}${map === undefined ? '' : `/maps/${encodeURIComponent(map)}`}`

  server.registerTool(
    'get_atlas',
    {
      title: "Lire l'atlas",
      description:
        "L'atlas de la campagne : les cartes (monde et campagne) rangées région › lieu › rencontre, et sur chacune les " +
        'scènes épinglées, les PNJ, indices et combats posés ; les lieux de l\'histoire et leur épingle ; les scènes sans ' +
        "épingle ; les alertes de placement. Puis la frise de l'histoire : pour chaque acte (un acte = une soirée), les " +
        "cartes que ses scènes traversent dans l'ordre du chemin — une case de plusieurs cartes est un embranchement " +
        "(des scènes au choix) — et les cartes à créer (carte_à_créer_pour : le lieu d'une scène qui n'a pas de carte " +
        "à lui) ; enfin les cartes hors de l'histoire. Pour voir une carte, get_map_text.",
      inputSchema: { campaign_id: campaignId },
      annotations: read,
    },
    ({ campaign_id }) =>
      guarded(async () => text(atlasReport(await promptus().request<Atlas>('GET', `${path(campaign_id)}/atlas`)))),
  )

  server.registerTool(
    'get_map_text',
    {
      title: 'Lire une carte en texte',
      description:
        'Une carte en caractères : axes numérotés pour nommer une case « x,y », le sol de chaque glyphe (mur, eau, ' +
        'terrain), et un symbole sur chaque élément posé (épingles, PNJ, indices, combats, portes, sorties, décors, ' +
        'objets, départs, étiquettes) avec sa légende. Une grande carte se lit par zones (x, y, w, h ; 80 × 50 au plus).',
      inputSchema: {
        campaign_id: campaignId,
        map_id: mapId,
        x: z.number().int().min(0).optional().describe('Colonne du coin haut gauche de la zone.'),
        y: z.number().int().min(0).optional().describe('Rangée du coin haut gauche de la zone.'),
        w: z.number().int().min(1).max(80).optional().describe('Largeur de la zone, en cases.'),
        h: z.number().int().min(1).max(50).optional().describe('Hauteur de la zone, en cases.'),
      },
      annotations: read,
    },
    ({ campaign_id, map_id, x, y, w, h }) =>
      guarded(async () => {
        const query = new URLSearchParams()
        for (const [k, v] of Object.entries({ x, y, w, h })) if (v !== undefined) query.set(k, String(v))
        const q = query.size ? `?${query}` : ''
        return text(sketchText(await promptus().request<Sketch>('GET', `${path(campaign_id, map_id)}/sketch${q}`)))
      }),
  )

  server.registerTool(
    'ask_map',
    {
      title: 'Questions au moteur',
      description:
        'Pose une question au moteur de règles sur une carte, avec des références par nom : « path » (le chemin le moins ' +
        'cher à pied entre from et to, son coût en cases de déplacement et en mètres), « sight » (from voit-il to, et ' +
        "quel couvert), « view » (ce que voient les joueurs depuis from : la carte des joueurs, le reste dans le brouillard).",
      inputSchema: {
        campaign_id: campaignId,
        map_id: mapId,
        question: z.enum(['path', 'sight', 'view']).describe('path, sight ou view.'),
        from: reference('Le point de départ, ou d\'où l\'on regarde'),
        to: reference("L'arrivée, ou ce que l'on regarde (pas pour view)").optional(),
      },
      annotations: read,
    },
    ({ campaign_id, map_id, question, from, to }) =>
      guarded(async () => {
        if (question !== 'view' && !to) {
          throw new PromptusError(0, 'MISSING_TO', `La question « ${question} » demande aussi « to ».`)
        }
        const body = question === 'view' ? { question, from } : { question, from, to }
        const a = await promptus().request<Json>('POST', `${path(campaign_id, map_id)}/ask`, body)
        const name = (t: unknown) => (t as { name: string }).name
        if (a.question === 'path') {
          const route = (a.path as Cell[]) ?? []
          return text(
            route.length
              ? {
                  de: name(a.from),
                  à: name(a.to),
                  coût: `${a.cost} case(s) de déplacement`,
                  mètres: a.meters,
                  chemin: cells(route, 80),
                }
              : { de: name(a.from), à: name(a.to), chemin: 'aucun chemin à pied' },
          )
        }
        if (a.question === 'sight') {
          return text({
            de: `${name(a.from)} ${at(a.viewer as Cell)}`,
            vers: `${name(a.to)} ${at(a.target as Cell)}`,
            voit: a.visible ? 'oui' : 'non',
            couvert: { none: 'aucun', half: 'moitié', three_quarters: 'trois quarts', total: 'total' }[a.cover as string],
            distance: `${a.cells} case(s), ${a.meters} m`,
          })
        }
        const light = { dark: 'obscurité', dim: 'pénombre', bright: 'pleine lumière' }[a.light as string]
        return text(
          `Depuis ${name(a.from)} ${at(a.viewer as Cell)}, les joueurs voient ${a.seen} case(s) (${light}). ` +
            'Ce qu\'ils ne voient pas est dans le brouillard.\n\n' +
            sketchText(a.sketch as Sketch),
        )
      }),
  )

  const placement = z.object({
    what: z.enum(['scene', 'place', 'npc', 'clue', 'fight']).describe("Ce que l'on pose : l'épingle d'une scène ou d'un lieu, un PNJ, un indice, le combat d'une scène."),
    id: z.string().min(1).describe('La scène, le lieu, le PNJ ou l\'indice (identifiant de get_campaign).'),
    scene: z.string().optional().describe('Pour un PNJ : la scène où il est là (sinon toutes les scènes jouées sur cette carte).'),
    near: reference('Près de quoi').optional(),
    along: z
      .object({ from: reference('Début du chemin'), to: reference('Fin du chemin') })
      .optional()
      .describe('Sur le chemin à pied entre deux références (à la place de near).'),
  })

  server.registerTool(
    'place_on_map',
    {
      title: 'Poser sur une carte',
      description: PLACE_HELP,
      inputSchema: {
        campaign_id: campaignId,
        map_id: mapId,
        placements: z.array(placement).min(1).max(50).describe('Les placements, dans l\'ordre, tout ou rien (50 au plus).'),
        title: z.string().max(200).optional().describe('Titre court du lot de propositions, lu par le MJ.'),
        proposal_id: z.string().optional().describe("Identifiant d'un lot encore en attente auquel ajouter ces placements."),
      },
      annotations: write,
    },
    ({ campaign_id, map_id, placements, title, proposal_id }) =>
      guarded(async () => {
        const r = await promptus().request<PlaceResult>('POST', `${path(campaign_id, map_id)}/place`, {
          placements,
          ...(title ? { title } : {}),
          ...(proposal_id ? { proposal: proposal_id } : {}),
        })
        return text(placeReport(r))
      }),
  )

  server.registerTool(
    'edit_map',
    {
      title: 'Redessiner une carte',
      description: OPS_HELP,
      inputSchema: {
        campaign_id: campaignId,
        map_id: mapId,
        ops: z.array(z.record(z.string(), z.unknown())).min(1).max(200).describe('Les opérations, dans l\'ordre, tout ou rien.'),
        title: z.string().max(200).optional().describe('Titre court du lot de propositions, lu par le MJ.'),
        proposal_id: z.string().optional().describe("Identifiant d'un lot encore en attente auquel ajouter ces opérations."),
      },
      annotations: write,
    },
    ({ campaign_id, map_id, ops, title, proposal_id }) =>
      guarded(async () => {
        const where = path(campaign_id, map_id)
        const send = (revision: number) =>
          promptus().request<OpsResult>('POST', `${where}/ops`, {
            revision,
            ops,
            ...(title ? { title } : {}),
            ...(proposal_id ? { proposal: proposal_id } : {}),
          })
        const { revision } = await promptus().request<MapRead>('GET', where)
        let r: OpsResult
        try {
          r = await send(revision)
        } catch (err) {
          const fresh = err instanceof PromptusError && err.code === 'MAP_STALE' ? (err.fresh as MapRead | undefined) : undefined
          if (fresh?.revision === undefined) throw err
          r = await send(fresh.revision)
        }
        if (r.status === 'proposed') {
          return text({
            status: 'proposé',
            why: WHY[r.why] ?? r.why,
            proposal: { id: r.proposal.id, title: r.proposal.prompt, items: r.proposal.items.length },
            next: 'En attente du MJ dans Promptus. list_proposals dira ce qu\'il garde.',
          })
        }
        return text({ status: 'appliqué', revision: r.revision })
      }),
  )
}
