/**
 * The tools Claude sees. Descriptions are in French (the GM's language);
 * every write goes through the server, which validates it and applies it
 * whole or not at all. Nothing here reaches players: the GM still
 * reveals content at the table, from Promptus.
 */

import type { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js'
import type { CallToolResult } from '@modelcontextprotocol/sdk/types.js'
import { z } from 'zod'
import { type Promptus, PromptusError } from './promptus'

type Json = Record<string, unknown>

interface Issue {
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

const campaignId = z.string().min(1).describe("Identifiant de la campagne (donné par list_campaigns).")

function text(value: unknown): CallToolResult {
  return {
    content: [{ type: 'text', text: typeof value === 'string' ? value : JSON.stringify(value, null, 2) }],
  }
}

function failure(err: unknown): CallToolResult {
  const message = err instanceof PromptusError
    ? err.message
    : `Erreur inattendue du serveur MCP Promptus : ${err instanceof Error ? err.message : String(err)}`
  return { isError: true, content: [{ type: 'text', text: message }] }
}

/** Run `work`, turning any failure into a tool error the GM can read. */
async function guarded(work: () => Promise<CallToolResult>): Promise<CallToolResult> {
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

const EDITS_HELP = `Applique des modifications ciblées à l'histoire d'une campagne, par identifiant, tout ou rien : \
si une seule modification ne passe pas, aucune n'est appliquée et l'erreur dit laquelle (son index). \
Rien n'atteint les joueurs : le MJ révèle toujours depuis Promptus.
Chaque modification est un objet :
- { "op": "set", "target": "<id ou bible ou campaign>", "field": "<champ, pointé pour aller dedans>", "value": <valeur> } \
— par ex. { "op": "set", "target": "sc_quai", "field": "summary", "value": "…" }, \
{ "op": "set", "target": "pnj_morel", "field": "stats.hit_points", "value": 12 }; "value": null efface le champ.
- { "op": "add", "kind": "<party_member|act|front|node|revelation|clue|npc|adversary|location|item|faction|goal>", "value": { "id": "…", … } } \
— par ex. un indice : { "op": "add", "kind": "clue", "value": { "id": "cl_…", "revelation": "…", "node": "…", "text": "…" } }
- { "op": "remove", "target": "<id>" }
Les champs suivent le format de campagne de Promptus (en snake_case : read_aloud, key_points, player_hook…). \
Lis d'abord la scène (get_scene) ou la campagne (get_campaign) pour connaître les identifiants. \
La réponse donne ce qui a changé et le rapport du validateur après coup.`

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
        const acts = await promptus().request<unknown>('GET', `/campaigns/${id}/readiness`)
        return text({ issues: issueReport(detail.issues, 100), acts })
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
      },
      annotations: { readOnlyHint: false, destructiveHint: false, idempotentHint: false, openWorldHint: false },
    },
    ({ campaign_id, edits }) =>
      guarded(async () => {
        const result = await promptus().request<{ campaign: CampaignDetail; changes: unknown }>(
          'POST',
          `/campaigns/${encodeURIComponent(campaign_id)}/story/edits`,
          { edits },
        )
        return text({ changes: result.changes, issues: issueReport(result.campaign.issues) })
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
    'import_campaign_yaml',
    {
      title: 'Remplacer la campagne par un YAML',
      description:
        "Remplace toute l'histoire d'une campagne existante par ce fichier YAML (format de Promptus, tel que donné par " +
        "export_campaign_yaml). Tout ce qui n'est pas dans le fichier disparaît : pour une retouche, préfère " +
        'apply_story_edits. Un YAML illisible est refusé sans rien changer ; la réponse donne le rapport du validateur.',
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
