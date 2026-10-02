// Co-MJ en direct : le LLM reçoit un contexte compact (bible, scène en cours,
// PNJ présents, menaces, indices, journal récent) et propose une narration,
// des répliques et des suggestions applicables. Rien n'atteint les joueurs
// sans validation du MJ. Fonctions pures, testables sans réseau.

import { z } from "zod";
import { PHASE_LABELS, type PhaseId } from "@/lib/engine/catalog";
import type { Ruleset } from "@/lib/engine/ruleset";
import type { CampaignStory } from "@/lib/engine/story";
import type { EntityState, EntityType, InitiativeEntry } from "@/lib/engine/types";
import { frontStep, isRevelationKnown, sceneTriggers, type WorldState } from "@/lib/engine/world";
import type { LlmMessage } from "@/lib/ai/llm";

export const COPILOT_KINDS = ["describe", "npc", "consequence", "next", "free"] as const;
export type CopilotKind = (typeof COPILOT_KINDS)[number];

export const COPILOT_KIND_LABELS: Record<CopilotKind, string> = {
  describe: "Décrire la situation",
  npc: "Réplique de PNJ",
  consequence: "Conséquences",
  next: "Et ensuite ?",
  free: "Question libre",
};

export interface CopilotRequest {
  kind: CopilotKind;
  /** PNJ qui parle (kind = npc). */
  npcId?: string;
  /** Précision du MJ : ce que disent les joueurs, la question… */
  prompt?: string;
}

export interface CopilotEntity {
  id: string;
  name: string;
  type: EntityType;
  description: string | null;
  attributes: Record<string, unknown>;
}

export interface CopilotContextInput {
  campaignName: string;
  story: CampaignStory;
  world: WorldState;
  ruleset: Ruleset;
  entities: CopilotEntity[];
  session: { phase: PhaseId; combatRound: number; activeTurnIndex: number; initiativeOrder: InitiativeEntry[] };
  participants: { entityId: string; state: EntityState }[];
  /** Journal récent, du plus ancien au plus récent. */
  timeline: string[];
  pendingRequests: string[];
  /** Résumés MJ des sessions précédentes, de la plus ancienne à la plus récente. */
  previousRecaps?: string[];
}

// ----------------------------------------------------------------------------
// Réponse
// ----------------------------------------------------------------------------

/** Suggestions que le MJ applique d'un clic (identifiants vérifiés). */
export const CopilotActionSchema = z.discriminatedUnion("type", [
  z.object({ type: z.literal("fire_trigger"), sceneId: z.string(), triggerId: z.string() }),
  z.object({ type: z.literal("reveal_clue"), clueId: z.string() }),
  z.object({ type: z.literal("advance_front"), frontId: z.string() }),
  z.object({ type: z.literal("enter_scene"), sceneId: z.string() }),
  z.object({ type: z.literal("reveal_entity"), entityId: z.string() }),
]);
export type CopilotAction = z.infer<typeof CopilotActionSchema>;

const ResponseSchema = z.object({
  narration: z.string().default(""),
  npcLines: z
    .array(z.object({ npcId: z.string().optional(), speaker: z.string().optional(), text: z.string() }))
    .default([]),
  suggestions: z
    .array(z.object({ label: z.string(), why: z.string().optional(), action: z.unknown().optional() }))
    .default([]),
  /** Conseil réservé au MJ (jamais montré aux joueurs). */
  gmNote: z.string().optional(),
});

export interface CopilotAnswer {
  narration: string;
  npcLines: { npcId?: string; speaker: string; text: string }[];
  suggestions: { label: string; why?: string; action?: CopilotAction }[];
  gmNote?: string;
}

/**
 * Valide la réponse du modèle : champs manquants tolérés, actions dont les
 * identifiants n'existent pas retirées (la suggestion reste, sans bouton).
 */
export function sanitizeAnswer(raw: unknown, story: CampaignStory, entities: CopilotEntity[]): CopilotAnswer {
  const parsed = ResponseSchema.safeParse(raw);
  if (!parsed.success) throw new Error("Réponse du co-MJ illisible");
  const r = parsed.data;
  const byId = new Map(entities.map((e) => [e.id, e]));
  const valid = (a: CopilotAction): boolean => {
    switch (a.type) {
      case "fire_trigger":
        return !!story.scenes.find((s) => s.id === a.sceneId)?.triggers.some((t) => t.id === a.triggerId);
      case "reveal_clue":
        return story.clues.some((c) => c.id === a.clueId);
      case "advance_front":
        return story.fronts.some((f) => f.id === a.frontId);
      case "enter_scene":
        return story.scenes.some((s) => s.id === a.sceneId);
      case "reveal_entity":
        return byId.has(a.entityId);
    }
  };
  return {
    narration: r.narration.trim(),
    npcLines: r.npcLines
      .filter((l) => l.text.trim())
      .map((l) => {
        const npc = l.npcId ? byId.get(l.npcId) : undefined;
        return { npcId: npc?.id, speaker: npc?.name ?? l.speaker ?? "PNJ", text: l.text.trim() };
      }),
    suggestions: r.suggestions.slice(0, 5).map((s) => {
      const a = CopilotActionSchema.safeParse(s.action);
      return { label: s.label, why: s.why, action: a.success && valid(a.data) ? a.data : undefined };
    }),
    gmNote: r.gmNote?.trim() || undefined,
  };
}

// ----------------------------------------------------------------------------
// Contexte
// ----------------------------------------------------------------------------

const cut = (s: string | null | undefined, n: number) => (!s ? "" : s.length > n ? `${s.slice(0, n)}…` : s);

/** Contexte compact pour le modèle : seulement ce qui sert à la scène en cours. */
export function buildCopilotContext(input: CopilotContextInput): string {
  const { story, world, session } = input;
  const byId = new Map(input.entities.map((e) => [e.id, e]));
  const name = (id: string) => byId.get(id)?.name ?? id;
  const scene = story.scenes.find((s) => s.id === world.currentSceneId);
  const lines: string[] = [];

  lines.push(`# Campagne « ${input.campaignName} »`);
  lines.push(`Pitch : ${cut(story.bible.pitch, 600)}`);
  lines.push(`Ton : ${story.bible.tone}`);
  if (story.bible.truths.length) lines.push(`Vérités du monde : ${story.bible.truths.map((t) => cut(t, 200)).join(" | ")}`);
  if (story.bible.secrets.length) lines.push(`Secrets (MJ) : ${story.bible.secrets.map((t) => cut(t, 200)).join(" | ")}`);

  if (input.previousRecaps?.length) {
    lines.push("", "# Sessions précédentes (résumés MJ)");
    for (const r of input.previousRecaps) lines.push(cut(r, 1500));
  }

  lines.push("", "# Scène en cours");
  if (scene) {
    lines.push(`${scene.id} « ${scene.title} » (phase ${PHASE_LABELS[scene.phase]}, statut ${world.sceneStatus[scene.id] ?? "disponible"})`);
    lines.push(`Résumé MJ : ${scene.summary}`);
    lines.push(`Objectif : ${scene.objective}`);
    lines.push(`Texte lu aux joueurs : ${cut(scene.readAloud, 800)}`);
    if (scene.gmNotes) lines.push(`Notes MJ : ${cut(scene.gmNotes, 600)}`);
    if (scene.locationEntityId) lines.push(`Lieu : ${name(scene.locationEntityId)} — ${cut(byId.get(scene.locationEntityId)?.description, 300)}`);
    const npcs = [...scene.npcEntityIds, ...scene.monsterEntityIds].map((id) => byId.get(id)).filter((e) => !!e);
    if (npcs.length) {
      lines.push("Présents :");
      for (const e of npcs) {
        const a = e.attributes;
        const extra = [
          a.motivation ? `motivation : ${cut(String(a.motivation), 200)}` : "",
          a.secret ? `secret : ${cut(String(a.secret), 200)}` : "",
          a.voiceActorRecommended ? `voix : ${cut(String(a.voiceActorRecommended), 120)}` : "",
          world.revealedEntityIds.includes(e.id) ? "connu des joueurs" : "",
        ].filter(Boolean);
        lines.push(`- ${e.id} ${e.name} (${e.type}) : ${cut(e.description, 200)}${extra.length ? ` [${extra.join(" ; ")}]` : ""}`);
      }
    }
    const clues = story.clues.filter((c) => c.sceneId === scene.id);
    if (clues.length) {
      lines.push("Indices de la scène :");
      for (const c of clues) {
        const found = world.foundClueIds.includes(c.id);
        lines.push(`- ${c.id} ${found ? "(trouvé)" : "(à trouver)"} : ${cut(c.text, 200)} — découverte : ${c.discovery}`);
      }
    }
    const triggers = sceneTriggers(story, world);
    if (triggers.length) {
      lines.push("Déclencheurs :");
      for (const t of triggers) lines.push(`- ${scene.id}/${t.trigger.id} « ${t.trigger.label} » ${t.fired ? "(déjà tiré)" : t.ready ? "(prêt)" : "(conditions non remplies)"}`);
    }
    if (scene.exits.length) {
      lines.push("Sorties :");
      for (const x of scene.exits) {
        const to = story.scenes.find((s) => s.id === x.toSceneId);
        lines.push(`- ${x.toSceneId} « ${to?.title ?? "?"} » : ${x.label}`);
      }
    }
  } else {
    lines.push("Aucune scène en cours.");
    const start = story.scenes.find((s) => s.id === story.bible.startSceneId);
    if (start) lines.push(`Scène d’ouverture : ${start.id} « ${start.title} »`);
  }

  if (story.fronts.length) {
    lines.push("", "# Menaces (fronts)");
    for (const f of story.fronts) {
      const step = frontStep(world, f.id);
      const next = f.steps[step + 1];
      lines.push(`- ${f.id} « ${f.name} » (but : ${cut(f.goal, 150)}) : étape ${step + 1}/${f.steps.length}${next ? ` ; prochaine : ${next.label}` : " ; catastrophe atteinte"}`);
    }
  }

  const known = story.revelations.filter((r) => isRevelationKnown(story, world, r.id));
  const unknown = story.revelations.filter((r) => !isRevelationKnown(story, world, r.id));
  if (story.revelations.length) {
    lines.push("", "# Révélations");
    if (known.length) lines.push(`Connues des joueurs : ${known.map((r) => cut(r.statement, 160)).join(" | ")}`);
    if (unknown.length) lines.push(`Encore cachées : ${unknown.map((r) => `${r.id} ${cut(r.statement, 160)}`).join(" | ")}`);
  }

  lines.push("", "# Table");
  lines.push(`Phase : ${PHASE_LABELS[session.phase]}${session.combatRound > 0 ? `, round ${session.combatRound}` : ""}`);
  const active = session.combatRound > 0 ? session.initiativeOrder[session.activeTurnIndex] : undefined;
  if (active) lines.push(`Tour de : ${name(active.entityId)}`);
  for (const p of input.participants) {
    const e = byId.get(p.entityId);
    if (!e) continue;
    const hp = p.state.hp !== undefined ? ` PV ${p.state.hp}/${p.state.hpMax ?? "?"}` : "";
    const conds = p.state.conditions.length ? ` états : ${p.state.conditions.map((c) => c.conditionId).join(", ")}` : "";
    lines.push(`- ${e.id} ${e.name} (${e.type === "character" ? "PJ" : e.type})${hp}${conds}`);
  }
  if (input.timeline.length) {
    lines.push("", "# Derniers événements (du plus ancien au plus récent)");
    for (const t of input.timeline) lines.push(`- ${cut(t, 300)}`);
  }
  if (input.pendingRequests.length) {
    lines.push("", "# Demandes des joueurs en attente");
    for (const r of input.pendingRequests) lines.push(`- ${cut(r, 300)}`);
  }
  return lines.join("\n");
}

const SYSTEM = `Tu es le co-MJ d’une partie de jeu de rôle, en français. Tu aides le MJ humain en direct : il a le dernier mot.
Règles :
- Reste fidèle au scénario, aux vérités du monde et au ton. N’invente pas de PNJ ou de lieux majeurs : improvise dans les détails.
- La narration s’adresse aux joueurs : 2 à 5 phrases au présent, sensorielles, à la deuxième personne du pluriel. Jamais de secret ni d’information MJ dedans.
- Les répliques de PNJ respectent leur motivation, leur secret (qu’ils protègent) et leur voix.
- Suggestions : 1 à 3 options concrètes pour le MJ. Quand c’est utile, joins une action applicable avec des identifiants EXISTANTS du contexte :
  {"type":"fire_trigger","sceneId","triggerId"} | {"type":"reveal_clue","clueId"} | {"type":"advance_front","frontId"} | {"type":"enter_scene","sceneId"} | {"type":"reveal_entity","entityId"}
- Respecte les règles du système : ne tranche pas les jets à la place des dés.
Réponds UNIQUEMENT avec un objet JSON :
{"narration": "texte pour les joueurs (ou vide)", "npcLines": [{"npcId": "ent_…", "text": "réplique"}], "suggestions": [{"label": "…", "why": "…", "action": {…}}], "gmNote": "conseil bref pour le MJ (facultatif)"}`;

function instruction(req: CopilotRequest, input: CopilotContextInput): string {
  const extra = req.prompt?.trim() ? `\nPrécision du MJ : ${req.prompt.trim()}` : "";
  switch (req.kind) {
    case "describe":
      return `Décris la situation actuelle aux joueurs (narration), en tenant compte des derniers événements.${extra}`;
    case "npc": {
      const npc = input.entities.find((e) => e.id === req.npcId);
      return `Donne 1 à 3 répliques de ${npc ? `${npc.id} ${npc.name}` : "un PNJ présent"} en réponse à ce qui vient de se passer. Narration courte ou vide.${extra}`;
    }
    case "consequence":
      return `Quelles sont les conséquences des dernières actions des joueurs ? Narration de ce qui se passe, réactions des PNJ, et suggestions (indice à révéler, menace qui avance, déclencheur…).${extra}`;
    case "next":
      return `Les joueurs hésitent ou la scène s’essouffle. Propose comment relancer : pistes vers les indices manquants, menace qui se rapproche, scène suivante.${extra}`;
    case "free":
      return req.prompt?.trim() ? `Question du MJ : ${req.prompt.trim()}` : "Aide le MJ pour la suite.";
  }
}

export function copilotMessages(req: CopilotRequest, input: CopilotContextInput): LlmMessage[] {
  const rules = `Système de règles : « ${input.ruleset.name} » (test ${input.ruleset.check.dice}).`;
  return [
    { role: "system", content: SYSTEM },
    { role: "user", content: `${buildCopilotContext(input)}\n\n${rules}\n\n# Demande\n${instruction(req, input)}` },
  ];
}
