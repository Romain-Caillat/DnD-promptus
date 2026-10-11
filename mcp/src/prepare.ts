/**
 * The preparation tools (campaign/prepare-with-claude): the moments of
 * the board « Créer · la campagne de Romain (refonte) » as calls. A
 * campaign starts from one scene (the opening or the ending), Claude
 * proposes around it a tension and its clock, then scenes — forward, or
 * backwards from the end — and the GM keeps or discards each item in
 * Promptus. Everything but the very first scene goes through
 * `story/edits`, so it is proposed like any of Claude's writes
 * (platform/propose-edits-by-claude). The skill that walks the GM
 * through it is `mcp/skills/preparer-soiree/SKILL.md`, also served as
 * the MCP prompt `preparer_soiree`.
 */

import { readFileSync } from 'node:fs'
import { join } from 'node:path'
import type { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js'
import { z } from 'zod'
import { type Promptus, PromptusError } from './promptus'
import { WHY, campaignId, guarded, type Issue, issueReport, type ProposalView, text } from './tools'

type Json = Record<string, unknown>

interface CampaignDetail {
  id: string
  story: Json
  issues: Issue[]
}

type EditsResult =
  | { status: 'applied'; campaign: CampaignDetail; changes: unknown }
  | { status: 'proposed'; why: string; proposal: ProposalView; issues: Issue[] }

interface RuleSystem {
  id: string
  version: number
  name: string
}

/** `Le Quai de Port-Louis` → `le_quai_de_port_louis`: accents, case and punctuation aside. */
export function slugify(title: string): string {
  return (
    title
      .normalize('NFD')
      .replace(/[̀-ͯ]/g, '')
      .toLowerCase()
      .replace(/[^a-z0-9]+/g, '_')
      .replace(/^_+|_+$/g, '')
      .slice(0, 40) || 'sans_titre'
  )
}

/** Every id the story already uses, whatever its list. */
function ids(story: Json): Set<string> {
  const out = new Set<string>()
  for (const value of Object.values(story)) {
    if (!Array.isArray(value)) continue
    for (const entry of value as Json[]) if (typeof entry?.id === 'string') out.add(entry.id)
  }
  return out
}

/** `prefix_slug`, or `prefix_slug_2`… when the story already has it. */
export function freshId(prefix: string, title: string, taken: Set<string>): string {
  const base = `${prefix}_${slugify(title)}`
  if (!taken.has(base)) return base
  for (let n = 2; ; n++) if (!taken.has(`${base}_${n}`)) return `${base}_${n}`
}

function list(story: Json, key: string): Json[] {
  const value = story[key]
  return Array.isArray(value) ? (value as Json[]) : []
}

/** The answer of `story/edits` as Claude reads it. */
function editsReport(result: EditsResult, extra: Json): Json {
  if (result.status === 'proposed') {
    return {
      status: 'proposé',
      why: WHY[result.why] ?? result.why,
      ...extra,
      proposal: { id: result.proposal.id, title: result.proposal.prompt, items: result.proposal.items.length },
      issues: issueReport(result.issues),
      next: 'En attente du MJ dans Promptus : il garde ou jette chaque élément. list_proposals dira lesquels.',
    }
  }
  return { status: 'appliqué', ...extra, issues: issueReport(result.campaign.issues) }
}

const sceneFields = {
  title: z.string().min(1).max(120).describe('Titre de la scène, court (« La bagarre du quai »).'),
  summary: z.string().max(2000).describe('Pour le MJ, une ou deux phrases : ce qui s\'y passe et ce qui s\'y joue.'),
  read_aloud: z.string().max(4000).optional().describe('Le texte lu aux joueurs en entrant, au présent.'),
  hook: z.string().max(2000).optional().describe("L'événement qui met la scène en mouvement."),
  flow: z.string().max(4000).optional().describe('Comment la scène se déroule.'),
  location: z.string().optional().describe("Identifiant d'un lieu de la campagne, s'il existe déjà."),
}

const SKILL_PATH = join(import.meta.dir, '..', 'skills', 'preparer-soiree', 'SKILL.md')

/** The skill's text without its front matter: what the prompt hands Claude. */
export function skillText(): string {
  const raw = readFileSync(SKILL_PATH, 'utf8')
  return raw.replace(/^---\n[\s\S]*?\n---\n+/, '')
}

export function registerPrepareTools(server: McpServer, promptus: () => Promptus) {
  const read = { readOnlyHint: true, openWorldHint: false }
  const write = { readOnlyHint: false, destructiveHint: false, idempotentHint: false, openWorldHint: false }

  server.registerPrompt(
    'preparer_soiree',
    {
      title: 'Préparer ma prochaine soirée',
      description:
        "Préparer une campagne en conversation, d'une première scène à « Jouable » : l'univers et le ton, " +
        'le lieu, les PNJ, les indices, la tension et son horloge, les scènes, les règles, puis le titre et l\'accroche.',
      argsSchema: {
        depart: z
          .string()
          .optional()
          .describe("D'où l'on part : une idée, le nom d'une campagne existante, ou le contenu d'un fichier."),
      },
    },
    ({ depart }) => ({
      messages: [
        {
          role: 'user',
          content: {
            type: 'text',
            text: `${skillText()}${depart ? `\n\n---\n\nDépart du MJ : ${depart}` : ''}`,
          },
        },
      ],
    }),
  )

  server.registerTool(
    'list_rule_systems',
    {
      title: 'Lister les systèmes de règles',
      description:
        'Les systèmes de règles que Promptus connaît (identifiant, version, nom) : une campagne en joue un. ' +
        'À proposer au MJ au premier combat de la préparation.',
      annotations: read,
    },
    () =>
      guarded(async () => {
        const systems = await promptus().request<RuleSystem[]>('GET', '/rule-systems')
        return text(systems.map((s) => ({ id: s.id, version: s.version, name: s.name })))
      }),
  )

  server.registerTool(
    'start_campaign',
    {
      title: 'Commencer une campagne par une scène',
      description:
        "Crée une nouvelle campagne à partir d'une seule scène que le MJ a racontée — celle d'ouverture, ou celle de " +
        'fin si l\'on prépare à rebours — avec l\'univers et le ton que le MJ a confirmés. Le titre et l\'accroche ' +
        'peuvent rester provisoires : on les fixe à la fin (moment 6). Les règles se choisissent au premier combat ' +
        '(list_rule_systems) ; d\'ici là la campagne joue le premier système connu, changeable avec apply_story_edits ' +
        '({ "op": "set", "target": "campaign", "field": "rules", "value": { "id": "…", "version": 1 } }). ' +
        "La campagne arrive en préparation, rien n'atteint les joueurs. Pour partir d'une campagne existante, " +
        "utilise list_campaigns ; d'un fichier complet, create_campaign_from_yaml.",
      inputSchema: {
        title: z.string().min(1).max(120).describe('Titre de travail de la campagne.'),
        world: z.string().max(200).describe("L'univers en quelques mots (« Bretagne, 1718, la nuit sur le port »)."),
        tone: z.string().max(500).describe('Le ton confirmé par le MJ.'),
        pitch: z.string().max(2000).optional().describe('Le pitch, si le MJ en a un.'),
        themes: z.array(z.string().max(60)).max(8).optional().describe('Les thèmes, quelques mots chacun.'),
        rules: z
          .object({ id: z.string().min(1), version: z.number().int().positive() })
          .optional()
          .describe('Le système de règles, si le MJ l\'a déjà choisi (list_rule_systems).'),
        scene: z
          .object({
            ...sceneFields,
            ending: z
              .boolean()
              .optional()
              .describe('true si c\'est la scène de fin : les suivantes se préparent à rebours.'),
          })
          .describe('La scène racontée par le MJ.'),
      },
      annotations: write,
    },
    ({ title, world, tone, pitch, themes, rules, scene }) =>
      guarded(async () => {
        const system = rules ?? (await promptus().request<RuleSystem[]>('GET', '/rule-systems'))[0]
        if (!system) {
          throw new PromptusError(0, 'NO_RULE_SYSTEM', 'Promptus ne connaît aucun système de règles : impossible de créer la campagne.')
        }
        const sceneId = freshId('sc', scene.title, new Set())
        const node: Json = {
          id: sceneId,
          act: 'acte_1',
          title: scene.title,
          summary: scene.summary,
          ...(scene.read_aloud ? { read_aloud: scene.read_aloud } : {}),
          ...(scene.hook ? { hook: scene.hook } : {}),
          ...(scene.flow ? { flow: scene.flow } : {}),
          ...(scene.ending ? { gm_notes: 'Scène de fin : la préparation remonte depuis elle.' } : {}),
        }
        const story = {
          format: 1,
          id: slugify(title),
          title,
          world,
          rules: { id: system.id, version: system.version },
          bible: {
            pitch: pitch ?? '',
            tone,
            themes: themes ?? [],
            start_node: sceneId,
          },
          acts: [{ id: 'acte_1', title: 'Acte I', summary: '' }],
          nodes: [node],
        }
        // JSON is YAML: the import route reads it as a campaign file.
        const detail = await promptus().request<CampaignDetail>('POST', '/campaigns/import', {
          yaml: JSON.stringify(story, null, 2),
        })
        return text({
          id: detail.id,
          title,
          rules: rules ? story.rules : { ...story.rules, provisional: 'à confirmer au premier combat' },
          scene: { id: sceneId, ending: scene.ending === true },
          issues: issueReport(detail.issues),
          next: 'Propose autour de cette scène le lieu, deux PNJ, trois indices et une tension (propose_tension).',
        })
      }),
  )

  server.registerTool(
    'propose_tension',
    {
      title: 'Proposer une tension et son horloge',
      description:
        "Propose une tension (un front) : une menace avec son but et une horloge d'étapes — 6 par défaut, de 4 à 6 — " +
        "la dernière étant la catastrophe si personne ne l'arrête. Elle part au MJ comme proposition à garder ou jeter.",
      inputSchema: {
        campaign_id: campaignId,
        name: z.string().min(1).max(120).describe('Le nom de la tension (« La Compagnie rachète le port »).'),
        goal: z.string().max(500).describe('Ce que la menace veut obtenir.'),
        description: z.string().max(2000).optional().describe('Pour le MJ : qui la porte, comment elle avance.'),
        steps: z
          .array(z.object({ label: z.string().min(1).max(120), description: z.string().max(1000).optional() }))
          .min(4)
          .max(6)
          .describe("Les étapes de l'horloge dans l'ordre ; la dernière est la catastrophe."),
        title: z.string().max(200).optional().describe('Titre du lot de propositions.'),
        proposal_id: z.string().optional().describe("Lot encore en attente auquel l'ajouter."),
      },
      annotations: write,
    },
    ({ campaign_id, name, goal, description, steps, title, proposal_id }) =>
      guarded(async () => {
        const id = encodeURIComponent(campaign_id)
        const detail = await promptus().request<CampaignDetail>('GET', `/campaigns/${id}`)
        const front = {
          id: freshId('front', name, ids(detail.story)),
          name,
          goal,
          ...(description ? { description } : {}),
          steps: steps.map((s) => ({ label: s.label, description: s.description ?? '' })),
        }
        const result = await promptus().request<EditsResult>('POST', `/campaigns/${id}/story/edits`, {
          edits: [{ op: 'add', kind: 'front', value: front }],
          title: title ?? `Tension : ${name}`,
          ...(proposal_id ? { proposal: proposal_id } : {}),
        })
        return text(editsReport(result, { front: front.id, clock: steps.length }))
      }),
  )

  server.registerTool(
    'propose_scene',
    {
      title: 'Proposer une scène',
      description:
        "Propose une scène reliée à une scène existante, en un seul lot que le MJ garde ou jette élément par élément : " +
        '« after » la place après une scène (la scène existante gagne une sortie vers elle) ; « before » la place avant ' +
        '(elle sort vers la scène existante) — c\'est la préparation à rebours depuis la fin ; avec « opening », elle ' +
        "devient la scène d'ouverture. « clues » y place des indices gardés : c'est là que les joueurs les trouvent. " +
        "L'acte est celui de la scène de référence, sauf si « act » le dit.",
      inputSchema: {
        campaign_id: campaignId,
        scene: z.object(sceneFields).describe('La scène proposée.'),
        after: z.string().optional().describe('Identifiant de la scène qui mène à celle-ci.'),
        before: z.string().optional().describe('Identifiant de la scène à laquelle celle-ci mène (à rebours).'),
        exit_label: z.string().max(120).optional().describe('Le libellé de la sortie (« Suivre la lanterne »).'),
        opening: z.boolean().optional().describe("true : la scène devient l'ouverture de la campagne."),
        act: z.string().optional().describe("Identifiant de l'acte, si ce n'est pas celui de la scène de référence."),
        clues: z.array(z.string()).max(10).optional().describe('Identifiants des indices trouvés dans cette scène.'),
        title: z.string().max(200).optional().describe('Titre du lot de propositions.'),
        proposal_id: z.string().optional().describe("Lot encore en attente auquel l'ajouter."),
      },
      annotations: write,
    },
    ({ campaign_id, scene, after, before, exit_label, opening, act, clues, title, proposal_id }) =>
      guarded(async () => {
        if (!after && !before) {
          throw new PromptusError(0, 'SCENE_UNLINKED', 'Dis à quelle scène la relier : « after » (elle suit) ou « before » (elle précède).')
        }
        const id = encodeURIComponent(campaign_id)
        const detail = await promptus().request<CampaignDetail>('GET', `/campaigns/${id}`)
        const nodes = list(detail.story, 'nodes')
        const named = (sceneId: string) => {
          const n = nodes.find((x) => x.id === sceneId)
          if (!n) {
            const known = nodes.map((x) => `${String(x.id)} (${String(x.title)})`).join(', ')
            throw new PromptusError(404, 'NO_SUCH_SCENE', `Pas de scène « ${sceneId} ». Scènes connues : ${known || 'aucune'}.`)
          }
          return n
        }
        const ref = named((after ?? before)!)
        if (after && before) named(before)
        const sceneId = freshId('sc', scene.title, ids(detail.story))
        const label = exit_label ?? scene.title
        const node: Json = {
          id: sceneId,
          act: act ?? ref.act,
          title: scene.title,
          summary: scene.summary,
          ...(scene.read_aloud ? { read_aloud: scene.read_aloud } : {}),
          ...(scene.hook ? { hook: scene.hook } : {}),
          ...(scene.flow ? { flow: scene.flow } : {}),
          ...(scene.location ? { location: scene.location } : {}),
          ...(before ? { exits: [{ to: before, label: named(before).title }] } : {}),
        }
        const edits: Json[] = [{ op: 'add', kind: 'node', value: node }]
        if (after) {
          const prior = named(after)
          const exits = Array.isArray(prior.exits) ? (prior.exits as Json[]) : []
          edits.push({ op: 'set', target: after, field: 'exits', value: [...exits, { to: sceneId, label }] })
        }
        if (opening) edits.push({ op: 'set', target: 'bible', field: 'start_node', value: sceneId })
        const known = new Set(list(detail.story, 'clues').map((c) => c.id))
        for (const clue of clues ?? []) {
          if (!known.has(clue)) {
            throw new PromptusError(404, 'NO_SUCH_CLUE', `Pas d'indice « ${clue} » dans cette campagne (get_campaign les liste).`)
          }
          edits.push({ op: 'set', target: clue, field: 'node', value: sceneId })
        }
        const result = await promptus().request<EditsResult>('POST', `/campaigns/${id}/story/edits`, {
          edits,
          title: title ?? `Scène : ${scene.title}`,
          ...(proposal_id ? { proposal: proposal_id } : {}),
        })
        return text(editsReport(result, { scene: sceneId, direction: before ? 'à rebours' : 'en avant' }))
      }),
  )
}
