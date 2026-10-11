/**
 * The tools Claude sees. Descriptions are in French (the GM's language);
 * every write goes through the server, which validates it whole or not
 * at all, then **proposes** it to the GM (platform/propose-edits-by-
 * claude): the GM keeps or discards each item in Promptus, and Claude
 * reads the outcome and the GM's words with `list_proposals`. Only when
 * the GM gave Claude the hand does a write land directly. Nothing here
 * reaches players: the GM still reveals content at the table.
 */

import type { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js'
import type { CallToolResult } from '@modelcontextprotocol/sdk/types.js'
import { z } from 'zod'
import { type Promptus, PromptusError } from './promptus'

type Json = Record<string, unknown>

export interface Issue {
  severity: string
  code: string
  path: string
  detail: string
}

interface CampaignDetail {
  id: string
  story: Json
  issues: Issue[]
  validatedAt: string | null
  archivedAt: string | null
  updatedAt: string
}

export interface ItemView {
  key: string
  status: 'pending' | 'kept' | 'discarded' | 'refused'
  stale: boolean
  word?: string
  code?: string
  edit?: Json
  change?: Json
  map?: { map: string; name: string; ops: number }
}

export interface ProposalView {
  id: string
  prompt: string
  status: 'pending' | 'accepted' | 'rejected'
  items: ItemView[]
  changes: unknown[]
  feedback?: string
  createdAt: string
}

type EditsResult =
  | { status: 'applied'; campaign: CampaignDetail; changes: unknown }
  | { status: 'proposed'; why: string; proposal: ProposalView; issues: Issue[] }

/** Why a write was proposed, in the GM's words. */
export const WHY: Record<string, string> = {
  proposals: 'le MJ garde la main : il gardera ou jettera chaque élément dans Promptus',
  evening: 'une soirée est ouverte : rien ne s\'applique directement pendant le jeu',
  revealed: 'cela touche ce que les joueurs connaissent déjà',
  batch: 'ajouté au lot demandé',
}

const ITEM_STATUS: Record<ItemView['status'], string> = {
  pending: 'en attente',
  kept: 'gardé',
  discarded: 'jeté',
  refused: 'refusé (ne s\'appliquait plus)',
}

/** One of Claude's batches as Claude reads it back: what the GM did with each item, and why. */
export function proposalReport(p: ProposalView): Json {
  return {
    id: p.id,
    title: p.prompt,
    status: { pending: 'en attente', accepted: 'tranché, au moins un élément gardé', rejected: 'tout jeté' }[p.status],
    ...(p.feedback ? { word: p.feedback } : {}),
    items: p.items.map((i) => ({
      key: i.key,
      status: ITEM_STATUS[i.status],
      ...(i.word ? { word: i.word } : {}),
      ...(i.code ? { refused: i.code } : {}),
      ...(i.stale ? { stale: "ne s'applique plus à la campagne telle qu'elle est" } : {}),
      ...(i.edit ? { edit: i.edit } : {}),
      ...(i.map ? { map: `${i.map.name || i.map.map} (${i.map.ops} opération(s))` } : {}),
    })),
  }
}

export const campaignId = z.string().min(1).describe("Identifiant de la campagne (donné par list_campaigns).")

export function text(value: unknown): CallToolResult {
  return {
    content: [{ type: 'text', text: typeof value === 'string' ? value : JSON.stringify(value, null, 2) }],
  }
}

export function failure(err: unknown): CallToolResult {
  const message = err instanceof PromptusError
    ? err.message
    : `Erreur inattendue du serveur MCP Promptus : ${err instanceof Error ? err.message : String(err)}`
  return { isError: true, content: [{ type: 'text', text: message }] }
}

/** Run `work`, turning any failure into a tool error the GM can read. */
export async function guarded(work: () => Promise<CallToolResult>): Promise<CallToolResult> {
  try {
    return await work()
  } catch (err) {
    return failure(err)
  }
}

function list(story: Json, key: string): Json[] {
  const value = story[key]
  return Array.isArray(value) ? (value as Json[]) : []
}

function pick(entry: Json, ...keys: string[]): Json {
  const out: Json = {}
  for (const key of keys) {
    const value = entry[key]
    if (value !== undefined && value !== null && value !== '') out[key] = value
  }
  return out
}

/** The validator's report, errors first, capped so the context stays small. */
export function issueReport(issues: Issue[], max = 40): Json {
  const errors = issues.filter((i) => i.severity === 'error')
  const warnings = issues.filter((i) => i.severity !== 'error')
  return {
    errors: errors.length,
    warnings: warnings.length,
    list: [...errors, ...warnings].slice(0, max),
    ...(issues.length > max ? { omitted: issues.length - max } : {}),
  }
}

/** The campaign at a glance: ids, titles and one-line summaries, no full text. */
export function summarize(detail: CampaignDetail): Json {
  const story = detail.story
  const bible = (story.bible ?? {}) as Json
  return {
    id: detail.id,
    title: story.title,
    world: story.world,
    rules: story.rules,
    validated: detail.validatedAt !== null,
    archived: detail.archivedAt !== null,
    updatedAt: detail.updatedAt,
    bible: pick(bible, 'pitch', 'tone', 'themes', 'player_hook', 'start_node'),
    party: list(story, 'party').map((p) => pick(p, 'id', 'name', 'player', 'concept', 'class')),
    acts: list(story, 'acts').map((a) => pick(a, 'id', 'title', 'summary')),
    fronts: list(story, 'fronts').map((f) => ({
      ...pick(f, 'id', 'name', 'goal'),
      steps: Array.isArray(f.steps) ? f.steps.length : 0,
    })),
    scenes: list(story, 'nodes').map((n) => ({
      ...pick(n, 'id', 'act', 'title', 'summary', 'optional', 'location'),
      exits: Array.isArray(n.exits) ? (n.exits as Json[]).map((e) => e.to) : [],
    })),
    revelations: list(story, 'revelations').map((r) => pick(r, 'id', 'statement', 'importance')),
    clues: list(story, 'clues').map((c) => pick(c, 'id', 'revelation', 'node')),
    npcs: list(story, 'npcs').map((n) => pick(n, 'id', 'name', 'title', 'location', 'faction')),
    adversaries: list(story, 'adversaries').map((a) => pick(a, 'id', 'name')),
    locations: list(story, 'locations').map((l) => pick(l, 'id', 'name', 'parent')),
    items: list(story, 'items').map((i) => pick(i, 'id', 'name', 'rarity')),
    factions: list(story, 'factions').map((f) => pick(f, 'id', 'name')),
    goals: list(story, 'goals').map((g) => pick(g, 'id', 'title')),
    issues: issueReport(detail.issues),
  }
}

/** One scene in full, with the clues placed in it and who is there. */
export function scene(detail: CampaignDetail, sceneId: string): Json | null {
  const story = detail.story
  const node = list(story, 'nodes').find((n) => n.id === sceneId)
  if (!node) return null
  const npcs = new Map(list(story, 'npcs').map((n) => [n.id, n]))
  const present = Array.isArray(node.npcs) ? (node.npcs as Json[]) : []
  return {
    scene: node,
    clues: list(story, 'clues').filter((c) => c.node === sceneId),
    npcs: present.map((slot) => {
      const npc = npcs.get(slot.npc)
      return { ...slot, ...(npc ? pick(npc, 'name', 'title', 'roleplay', 'wants', 'hides') : {}) }
    }),
    issues: detail.issues.filter((i) => i.path.includes(sceneId) || i.detail.includes(sceneId)),
  }
}

/**
 * « Jouable » (campaign/prepare-with-claude, moment 6): no validator
 * error and the first act's gauge full. The GM still declares it
 * playable in Promptus; this only says whether it is time to.
 */
export function playable(issues: Issue[], acts: { title: string; ready: boolean }[]): Json {
  const errors = issues.filter((i) => i.severity === 'error').length
  const first = acts[0]
  const missing: string[] = []
  if (errors > 0) missing.push(`${errors} erreur(s) du validateur`)
  if (!first) missing.push('aucun acte')
  else if (!first.ready) missing.push(`la jauge de « ${first.title} » n'est pas pleine`)
  return missing.length === 0
    ? { verdict: 'Jouable', next: "Fixe avec le MJ le titre et l'accroche, puis il déclare la campagne jouable dans Promptus." }
    : { verdict: 'Pas encore jouable', missing }
}

const EDITS_HELP = `Propose des modifications ciblées de l'histoire d'une campagne, par identifiant, vérifiées tout ou rien : \
si une seule modification ne passe pas, rien n'est proposé et l'erreur dit laquelle (son index). \
Par défaut elles forment un lot de propositions que le MJ garde ou jette, une à une, dans Promptus (l'Atlas les \
dessine en pointillés) : rien ne change avant. Donne un titre court au lot (« title ») ; pour ajouter à un lot \
encore en attente, passe son identifiant (« proposal_id »). list_proposals dit ce que le MJ a gardé et le mot qu'il \
a laissé sur ce qu'il a jeté : propose autre chose en tenant compte de ce mot. Si le MJ t'a donné la main \
(« Claude a la main »), elles s'appliquent directement — sauf pendant une soirée ouverte ou sur ce que les joueurs \
connaissent déjà, où elles restent proposées. Rien n'atteint les joueurs : le MJ révèle toujours depuis Promptus.
Chaque modification est un objet :
- { "op": "set", "target": "<id ou bible ou campaign>", "field": "<champ, pointé pour aller dedans>", "value": <valeur> } \
— par ex. { "op": "set", "target": "sc_quai", "field": "summary", "value": "…" }, \
{ "op": "set", "target": "pnj_morel", "field": "stats.hit_points", "value": 12 }; "value": null efface le champ. \
Pour poser quelque chose sur une carte, préfère place_on_map : Promptus y choisit lui-même une case libre, au sec et \
atteignable ; les coordonnées brutes ci-dessous ne servent qu'à corriger à la main. \
Épingler un lieu sur une carte (ses scènes y apparaissent) : \
{ "op": "set", "target": "lieu_quai", "field": "anchor", "value": { "map": "<id de carte>", "at": [x, y] } }. \
Poser un PNJ sur une carte (toutes les scènes, ou une seule avec "scene") : \
{ "op": "set", "target": "pnj_morel", "field": "spots", "value": [{ "map": "<id de carte>", "at": [x, y], "scene": "sc_quai" }] } \
(la liste entière, relue d'abord) ; un indice : { "op": "set", "target": "cl_…", "field": "anchor", "value": { "map": "…", "at": [x, y] } } ; \
le combat d'une scène : { "op": "set", "target": "sc_quai", "field": "encounter.anchor", "value": { "map": "…", "at": [x, y] } }.
- { "op": "add", "kind": "<party_member|act|front|node|revelation|clue|npc|adversary|location|item|faction|goal>", "value": { "id": "…", … } } \
— par ex. un indice : { "op": "add", "kind": "clue", "value": { "id": "cl_…", "revelation": "…", "node": "…", "text": "…" } }
- { "op": "remove", "target": "<id>" }
Les champs suivent le format de campagne de Promptus (en snake_case : read_aloud, key_points, player_hook…). \
Lis d'abord la scène (get_scene) ou la campagne (get_campaign) pour connaître les identifiants. \
La réponse dit si c'est proposé ou appliqué, ce qui change et le rapport du validateur après coup.`

export function registerTools(server: McpServer, promptus: () => Promptus) {
  const read = { readOnlyHint: true, openWorldHint: false }

  server.registerTool(
    'list_campaigns',
    {
      title: 'Lister les campagnes',
      description: 'Liste les campagnes du MJ dans Promptus : identifiant, titre, univers, règles, archivée ou non, dernière activité.',
      annotations: read,
    },
    () =>
      guarded(async () => {
        const campaigns = await promptus().request<Json[]>('GET', '/campaigns')
        return text(
          campaigns.map((c) => ({
            ...pick(c, 'id', 'title', 'world', 'rules', 'lastActivityAt'),
            archived: c.archivedAt !== null && c.archivedAt !== undefined,
          })),
        )
      }),
  )

  server.registerTool(
    'get_campaign',
    {
      title: 'Lire une campagne',
      description:
        "Résumé d'une campagne : bible, actes, fronts, scènes (identifiant, titre, résumé, sorties), révélations, " +
        "indices, PNJ, lieux, objets, factions, et le rapport du validateur (erreurs et avertissements). " +
        'Pour le texte complet d\'une scène, utilise get_scene ; pour tout le fichier, export_campaign_yaml.',
      inputSchema: { campaign_id: campaignId },
      annotations: read,
    },
    ({ campaign_id }) =>
      guarded(async () => {
        const detail = await promptus().request<CampaignDetail>('GET', `/campaigns/${encodeURIComponent(campaign_id)}`)
        return text(summarize(detail))
      }),
  )

  server.registerTool(
    'get_scene',
    {
      title: 'Lire une scène',
      description:
        "Une scène (un nœud de l'histoire) en entier : texte lu aux joueurs, déroulé, jets prévus, rencontre, butin, " +
        'sorties, notes du MJ ; avec les indices placés dans la scène, les PNJ présents et les alertes qui la concernent.',
      inputSchema: {
        campaign_id: campaignId,
        scene_id: z.string().min(1).describe('Identifiant de la scène (par ex. sc_auberge), donné par get_campaign.'),
      },
      annotations: read,
    },
    ({ campaign_id, scene_id }) =>
      guarded(async () => {
        const detail = await promptus().request<CampaignDetail>('GET', `/campaigns/${encodeURIComponent(campaign_id)}`)
        const found = scene(detail, scene_id)
        if (!found) {
          const known = list(detail.story, 'nodes').map((n) => `${String(n.id)} (${String(n.title)})`)
          return failure(
            new PromptusError(404, 'NO_SUCH_SCENE', `Pas de scène « ${scene_id} » dans cette campagne. Scènes connues : ${known.join(', ') || 'aucune'}.`),
          )
        }
        return text(found)
      }),
  )

  server.registerTool(
    'get_readiness',
    {
      title: 'Alertes de cohérence',
      description:
        "Ce qui manque avant de jouer : le rapport du validateur (références cassées, règle des trois indices, scènes " +
        "inaccessibles, savoir jamais donné…) et la jauge de chaque acte (champs de scène manquants, chemins vers les " +
        'révélations, accroches des personnages, rencontres chiffrées, combats simulés).',
      inputSchema: { campaign_id: campaignId },
      annotations: read,
    },
    ({ campaign_id }) =>
      guarded(async () => {
        const id = encodeURIComponent(campaign_id)
        const detail = await promptus().request<CampaignDetail>('GET', `/campaigns/${id}`)
        const acts = await promptus().request<{ act: string; title: string; ready: boolean }[]>(
          'GET',
          `/campaigns/${id}/readiness`,
        )
        return text({ playable: playable(detail.issues, acts), issues: issueReport(detail.issues, 100), acts })
      }),
  )

  server.registerTool(
    'apply_story_edits',
    {
      title: "Modifier l'histoire",
      description: EDITS_HELP,
      inputSchema: {
        campaign_id: campaignId,
        edits: z
          .array(z.record(z.string(), z.unknown()))
          .min(1)
          .max(200)
          .describe('Les modifications, appliquées dans l\'ordre, tout ou rien (200 au plus).'),
        title: z
          .string()
          .max(200)
          .optional()
          .describe('Titre court du lot de propositions, lu par le MJ (par ex. « Peupler le cimetière »).'),
        proposal_id: z
          .string()
          .optional()
          .describe("Identifiant d'un lot encore en attente auquel ajouter ces modifications (donné par la réponse ou list_proposals)."),
      },
      annotations: { readOnlyHint: false, destructiveHint: false, idempotentHint: false, openWorldHint: false },
    },
    ({ campaign_id, edits, title, proposal_id }) =>
      guarded(async () => {
        const result = await promptus().request<EditsResult>(
          'POST',
          `/campaigns/${encodeURIComponent(campaign_id)}/story/edits`,
          { edits, ...(title ? { title } : {}), ...(proposal_id ? { proposal: proposal_id } : {}) },
        )
        if (result.status === 'proposed') {
          return text({
            status: 'proposé',
            why: WHY[result.why] ?? result.why,
            proposal: { id: result.proposal.id, title: result.proposal.prompt, items: result.proposal.items.length },
            changes: result.proposal.changes,
            issues: issueReport(result.issues),
            next: 'En attente du MJ dans Promptus. list_proposals dira ce qu\'il garde, et pourquoi il jette.',
          })
        }
        return text({ status: 'appliqué', changes: result.changes, issues: issueReport(result.campaign.issues) })
      }),
  )

  server.registerTool(
    'list_proposals',
    {
      title: 'Lire la réponse du MJ',
      description:
        "Les derniers lots de propositions de Claude dans une campagne, les plus récents d'abord : titre, état (en attente, " +
        'gardé, rejeté), et pour chaque élément ce que le MJ en a fait — gardé, jeté avec son mot, ou refusé parce qu\'il ' +
        "ne s'appliquait plus (la campagne avait changé). Lis-le avant de proposer à nouveau, et tiens compte des mots du MJ.",
      inputSchema: { campaign_id: campaignId },
      annotations: read,
    },
    ({ campaign_id }) =>
      guarded(async () => {
        const batches = await promptus().request<ProposalView[]>('GET', `/campaigns/${encodeURIComponent(campaign_id)}/proposals`)
        return text(batches.map(proposalReport))
      }),
  )

  server.registerTool(
    'export_campaign_yaml',
    {
      title: 'Exporter la campagne (YAML)',
      description: 'Le fichier complet de la campagne, au format YAML de Promptus (celui que import_campaign_yaml relit).',
      inputSchema: { campaign_id: campaignId },
      annotations: read,
    },
    ({ campaign_id }) =>
      guarded(async () => {
        const { yaml } = await promptus().request<{ yaml: string }>('GET', `/campaigns/${encodeURIComponent(campaign_id)}/export`)
        return text(yaml)
      }),
  )

  server.registerTool(
    'create_campaign_from_yaml',
    {
      title: 'Créer une campagne depuis un YAML',
      description:
        'Crée une nouvelle campagne du MJ à partir d\'un fichier YAML complet (format de Promptus, docs/campaign-format.md ; ' +
        'export_campaign_yaml en donne un exemple). Sert à mettre dans Promptus une campagne écrite avec le MJ. Rien ' +
        'n\'atteint les joueurs : la campagne arrive en préparation. Un YAML illisible est refusé sans rien créer ; la ' +
        'réponse donne l\'identifiant de la campagne et le rapport du validateur.',
      inputSchema: {
        yaml: z.string().min(1).describe('Le fichier de campagne complet, en YAML.'),
      },
      annotations: { readOnlyHint: false, destructiveHint: false, idempotentHint: false, openWorldHint: false },
    },
    ({ yaml }) =>
      guarded(async () => {
        const detail = await promptus().request<CampaignDetail>('POST', '/campaigns/import', { yaml })
        return text({ id: detail.id, title: detail.story.title, issues: issueReport(detail.issues) })
      }),
  )

  server.registerTool(
    'import_campaign_yaml',
    {
      title: 'Remplacer la campagne par un YAML',
      description:
        "Remplace toute l'histoire d'une campagne existante par ce fichier YAML (format de Promptus, tel que donné par " +
        "export_campaign_yaml). Possible seulement quand le MJ t'a donné la main (« Claude a la main »), hors soirée " +
        "ouverte et sans toucher ce que les joueurs connaissent : un fichier entier ne se propose pas élément par élément. " +
        "Tout ce qui n'est pas dans le fichier disparaît : pour une retouche, préfère apply_story_edits. Un YAML " +
        'illisible est refusé sans rien changer ; la réponse donne le rapport du validateur.',
      inputSchema: {
        campaign_id: campaignId,
        yaml: z.string().min(1).describe('Le fichier de campagne complet, en YAML.'),
      },
      annotations: { readOnlyHint: false, destructiveHint: true, idempotentHint: true, openWorldHint: false },
    },
    ({ campaign_id, yaml }) =>
      guarded(async () => {
        const detail = await promptus().request<CampaignDetail>(
          'PUT',
          `/campaigns/${encodeURIComponent(campaign_id)}/import`,
          { yaml },
        )
        return text({ id: detail.id, title: detail.story.title, issues: issueReport(detail.issues) })
      }),
  )
}
