/**
 * The PixelLab tools (campaign/import-pixellab-assets). Claude holds
 * both MCP servers: it reads here what the theme and the cast still
 * lack, announces the generations, generates at PixelLab once the GM
 * agrees, and hands each image's address to Promptus, which fetches it
 * itself, brings an element's colours onto the theme's palette and
 * keeps it pending: the GM keeps or discards it in Promptus.
 */

import type { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js'
import { z } from 'zod'
import type { Promptus } from './promptus'
import { campaignId, guarded, text } from './tools'

interface Need {
  kind: 'element' | 'character'
  subject: string
  name: string
  form: 'texture' | 'flat' | 'facade' | 'character'
  status: 'missing' | 'pending' | 'approved'
  generations: number
}

interface Needs {
  tileset: string | null
  palette: string[]
  needs: Need[]
  generations: number
}

/** How PixelLab should draw each form: the shape rule of `MEMORY.md` §2. */
export const HOW: Record<Need['form'], string> = {
  texture: "texture vue de face (view: side), sans perspective ni isométrie, qui se répète : posée sur un volume ou au sol",
  flat: 'image plate sur fond transparent, vue de côté, tournée vers la caméra',
  facade: "façade de bâtiment vue de face (view: side), sans perspective",
  character: '4 directions (sud, est, nord, ouest) et les animations repos, marche, attaque, coup reçu, chute',
}

const STATUS: Record<Need['status'], string> = {
  missing: 'manque',
  pending: 'en attente du MJ (ne pas regénérer)',
  approved: 'gardé par le MJ (ne pas regénérer)',
}

/** The needs as Claude reads them: what to generate, how, and the bill. */
export function needsReport(n: Needs): Record<string, unknown> {
  const missing = n.needs.filter((x) => x.status === 'missing')
  return {
    tileset: n.tileset,
    palette: n.palette,
    missing: missing.map((x) => ({
      kind: x.kind,
      subject: x.subject,
      name: x.name,
      how: HOW[x.form],
      generations: x.generations,
    })),
    already: n.needs.filter((x) => x.status !== 'missing').map((x) => ({ subject: x.subject, name: x.name, status: STATUS[x.status] })),
    generations: n.generations,
    next:
      missing.length === 0
        ? 'Rien ne manque : réutilise ce qui est là.'
        : `Annonce au MJ ${n.generations} génération(s) PixelLab et attends son accord avant de générer.`,
  }
}

export function registerPixellabTools(server: McpServer, promptus: () => Promptus) {
  server.registerTool(
    'get_theme_needs',
    {
      title: 'Ce qui manque au thème (PixelLab)',
      description:
        "Ce que le rendu HD-2D attend encore de PixelLab : pour une carte (map_id), les textures de ses sols, de ses murs, " +
        "des faces de ses accessoires en volume, les images plates de ses accessoires fins, et les personnages des PNJ " +
        "qui y sont posés ; pour un PNJ (npc_id), son personnage ; sans rien, tout le thème et tous les PNJ. Pour chacun : " +
        "manque, en attente ou gardé, comment le dessiner, et le coût en générations. Ce qui est en attente ou gardé ne " +
        "se regénère jamais. La palette du thème est à donner à PixelLab.",
      inputSchema: {
        campaign_id: campaignId,
        map_id: z.string().optional().describe("Identifiant d'une carte (get_atlas) : le lieu à habiller."),
        npc_id: z.string().optional().describe("Identifiant d'un PNJ ou d'un adversaire gardé."),
      },
      annotations: { readOnlyHint: true, openWorldHint: false },
    },
    ({ campaign_id, map_id, npc_id }) =>
      guarded(async () => {
        const q = new URLSearchParams()
        if (map_id) q.set('map', map_id)
        if (npc_id) q.set('npc', npc_id)
        const query = q.toString()
        const needs = await promptus().request<Needs>(
          'GET',
          `/campaigns/${encodeURIComponent(campaign_id)}/elements${query ? `?${query}` : ''}`,
        )
        return text(needsReport(needs))
      }),
  )

  const frame = z.object({
    url: z.string().min(1).describe("Adresse de l'image chez PixelLab (https), ou data:image/png;base64,…"),
    direction: z.string().describe('south, east, north, west (ou les huit de PixelLab).'),
    animation: z.string().optional().describe('idle (par défaut), walk, attack, hit, fall…'),
    index: z.number().int().min(0).optional().describe("Rang de l'image dans l'animation (0 par défaut)."),
  })

  server.registerTool(
    'import_pixellab_asset',
    {
      title: 'Déposer une image PixelLab',
      description:
        "Dépose dans Promptus une image générée chez PixelLab, en attente de validation par le MJ (rien n'atteint les " +
        'joueurs avant). Un élément du thème (kind « element ») : son emplacement tel que get_theme_needs le donne ' +
        '(« port-1718/material/pavés », « port-1718/wall », « port-1718/prop/caisse/side », « port-1718/prop/cordages », ' +
        '« port-1718/facade/taverne ») et l\'adresse de l\'image ; Promptus ramène ses couleurs sur la palette du thème. ' +
        'Un personnage (kind « character ») : l\'identifiant du PNJ et ses images (frames), une par direction et par ' +
        "image d'animation. Promptus télécharge lui-même, depuis PixelLab seulement. Un emplacement déjà rempli est " +
        "refusé : ne regénère pas, sauf si le MJ demande de remplacer (replace).",
      inputSchema: {
        campaign_id: campaignId,
        kind: z.enum(['element', 'character']),
        subject: z.string().min(1).describe("L'emplacement de l'élément, ou l'identifiant du PNJ."),
        url: z.string().optional().describe("L'image d'un élément."),
        frames: z.array(frame).max(80).optional().describe("Les images d'un personnage."),
        description: z.string().max(500).optional().describe('Ce que tu as demandé à PixelLab, pour le MJ.'),
        generations: z.number().int().min(0).optional().describe('Les générations PixelLab que cela a coûté.'),
        replace: z.boolean().optional().describe('Remplacer ce qui est déjà là (seulement si le MJ le demande).'),
      },
      annotations: { readOnlyHint: false, destructiveHint: false, idempotentHint: false, openWorldHint: true },
    },
    ({ campaign_id, kind, subject, url, frames, description, generations, replace }) =>
      guarded(async () => {
        const asset = await promptus().request<{ id: string; status: string; meta: { name?: string; frames?: unknown[] } }>(
          'POST',
          `/campaigns/${encodeURIComponent(campaign_id)}/media/import`,
          {
            kind,
            subject,
            ...(url ? { url } : {}),
            ...(frames ? { frames } : {}),
            ...(description ? { description } : {}),
            ...(generations !== undefined ? { generations } : {}),
            ...(replace ? { replace } : {}),
          },
        )
        return text({
          status: 'en attente du MJ',
          id: asset.id,
          name: asset.meta.name,
          ...(asset.meta.frames && asset.meta.frames.length > 0 ? { frames: asset.meta.frames.length } : {}),
          next: "Le MJ la garde ou la jette dans Promptus ; l'Atlas la montre déjà, marquée en attente.",
        })
      }),
  )
}
