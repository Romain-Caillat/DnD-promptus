// Continuité d'une campagne : ce qui a changé pendant une session (faits
// tirés des données, sans IA), récapitulatifs pour le MJ et pour les joueurs.
// Fonctions pures. Le LLM réécrit les faits en prose ; sans lui, le
// récapitulatif factuel reste utilisable tel quel.

import { z } from "zod";
import type { LlmMessage } from "@/lib/ai/llm";
import type { CampaignStory } from "@/lib/engine/story";
import type { EntityState, EntityType } from "@/lib/engine/types";
import { frontStep, isRevelationKnown, type WorldState } from "@/lib/engine/world";

export interface RecapFacts {
  durationMinutes: number;
  scenesVisited: { id: string; title: string; resolved: boolean }[];
  cluesFound: { id: string; text: string }[];
  revelationsLearned: { id: string; statement: string }[];
  frontsAdvanced: { id: string; name: string; from: number; to: number; step: string; total: number }[];
  entitiesRevealed: string[];
  combats: number;
  /** Adversaires à terre (nom visible des joueurs entre parenthèses). */
  fallen: { name: string; visibleName: string }[];
  party: { name: string; hp: number | null; hpMax: number | null; down: boolean }[];
  events: number;
}

export interface SessionRecap {
  facts: RecapFacts;
  /** Pour le MJ : déroulé, fils ouverts, pistes. Peut contenir des secrets. */
  gm: string;
  /** « Précédemment… » pour les joueurs : sans aucun secret. */
  players: string;
  source: "facts" | "llm";
  status: "draft" | "published";
  generatedAt: string;
  publishedAt?: string;
  /** Génération IA en cours ou en échec. */
  llm?: { status: "running" | "failed"; error?: string };
}

export interface FactsInput {
  story: CampaignStory;
  before: WorldState;
  after: WorldState;
  entities: { id: string; name: string; type: EntityType; visibility: string }[];
  states: { entityId: string; state: EntityState }[];
  /** Journal complet de la session (MJ). */
  timeline: string[];
  startedAt: Date;
  endedAt: Date;
}

const knownToPlayers = (e: FactsInput["entities"][number], world: WorldState) =>
  e.type === "character" || e.visibility === "public" || world.revealedEntityIds.includes(e.id);

export function sessionFacts(input: FactsInput): RecapFacts {
  const { story, before, after } = input;
  const byId = new Map(input.entities.map((e) => [e.id, e]));

  // Scènes jouées : statut changé pendant la session, ou scène en cours.
  const sceneIds = new Set<string>();
  for (const s of story.scenes) {
    if ((after.sceneStatus[s.id] ?? null) !== (before.sceneStatus[s.id] ?? null)) sceneIds.add(s.id);
  }
  if (before.currentSceneId) sceneIds.add(before.currentSceneId);
  if (after.currentSceneId) sceneIds.add(after.currentSceneId);
  const scenesVisited = story.scenes
    .filter((s) => sceneIds.has(s.id))
    .map((s) => ({ id: s.id, title: s.title, resolved: after.sceneStatus[s.id] === "resolved" }));

  const newClues = after.foundClueIds.filter((id) => !before.foundClueIds.includes(id));
  const cluesFound = story.clues.filter((c) => newClues.includes(c.id)).map((c) => ({ id: c.id, text: c.text }));
  const revelationsLearned = story.revelations
    .filter((r) => isRevelationKnown(story, after, r.id) && !isRevelationKnown(story, before, r.id))
    .map((r) => ({ id: r.id, statement: r.statement }));
  const frontsAdvanced = story.fronts
    .map((f) => ({ f, from: frontStep(before, f.id), to: frontStep(after, f.id) }))
    .filter((x) => x.to > x.from)
    .map(({ f, from, to }) => ({ id: f.id, name: f.name, from, to, step: f.steps[to]?.label ?? "", total: f.steps.length }));
  const entitiesRevealed = after.revealedEntityIds
    .filter((id) => !before.revealedEntityIds.includes(id))
    .map((id) => byId.get(id)?.name)
    .filter((n): n is string => !!n);

  const fallen: RecapFacts["fallen"] = [];
  const party: RecapFacts["party"] = [];
  for (const s of input.states) {
    const e = byId.get(s.entityId);
    if (!e) continue;
    const down = typeof s.state.hp === "number" && s.state.hp <= 0;
    if (e.type === "character") party.push({ name: e.name, hp: s.state.hp ?? null, hpMax: s.state.hpMax ?? null, down });
    else if (down) fallen.push({ name: e.name, visibleName: knownToPlayers(e, after) ? e.name : e.type === "npc" ? "un inconnu" : "un adversaire" });
  }

  return {
    durationMinutes: Math.max(0, Math.round((input.endedAt.getTime() - input.startedAt.getTime()) / 60_000)),
    scenesVisited,
    cluesFound,
    revelationsLearned,
    frontsAdvanced,
    entitiesRevealed,
    combats: input.timeline.filter((t) => t.startsWith("⚔️ Le combat commence")).length,
    fallen,
    party,
    events: input.timeline.length,
  };
}

const list = (items: string[]) => items.map((i) => `- ${i}`).join("\n");

/** Récapitulatifs factuels (sans IA), utilisables tels quels ou comme base. */
export function factsRecap(f: RecapFacts): { gm: string; players: string } {
  const gm: string[] = [];
  gm.push(`Durée : ${f.durationMinutes} min · ${f.events} événements · ${f.combats} combat(s).`);
  if (f.scenesVisited.length) gm.push(`Scènes :\n${list(f.scenesVisited.map((s) => `${s.title}${s.resolved ? " (résolue)" : ""}`))}`);
  if (f.cluesFound.length) gm.push(`Indices trouvés :\n${list(f.cluesFound.map((c) => c.text))}`);
  if (f.revelationsLearned.length) gm.push(`Révélations désormais connues :\n${list(f.revelationsLearned.map((r) => r.statement))}`);
  if (f.frontsAdvanced.length) {
    gm.push(`Menaces :\n${list(f.frontsAdvanced.map((x) => `${x.name} : étape ${x.to + 1}/${x.total} — ${x.step}`))}`);
  }
  if (f.entitiesRevealed.length) gm.push(`Révélé aux joueurs : ${f.entitiesRevealed.join(", ")}.`);
  if (f.fallen.length) gm.push(`Adversaires vaincus : ${f.fallen.map((x) => x.name).join(", ")}.`);
  const hurt = f.party.filter((p) => p.down || (p.hp !== null && p.hpMax !== null && p.hp < p.hpMax));
  if (hurt.length) gm.push(`État du groupe : ${hurt.map((p) => `${p.name} ${p.down ? "à terre" : `${p.hp}/${p.hpMax} PV`}`).join(", ")}.`);

  const players: string[] = [];
  const titles = f.scenesVisited.map((s) => s.title);
  players.push(titles.length ? `Précédemment : votre route vous a menés à ${titles.join(", puis ")}.` : "Précédemment : l’aventure a commencé.");
  if (f.cluesFound.length) players.push(`Vous avez découvert :\n${list(f.cluesFound.map((c) => c.text))}`);
  if (f.revelationsLearned.length) players.push(`Vous savez désormais :\n${list(f.revelationsLearned.map((r) => r.statement))}`);
  if (f.fallen.length) {
    const names = [...new Set(f.fallen.map((x) => x.visibleName))];
    players.push(`Au combat, vous avez abattu ${names.join(", ")}.`);
  }
  const down = f.party.filter((p) => p.down);
  if (down.length) players.push(`${down.map((p) => p.name).join(" et ")} ${down.length > 1 ? "sont" : "est"} à terre.`);
  return { gm: gm.join("\n\n"), players: players.join("\n\n") };
}

// ----------------------------------------------------------------------------
// Récapitulatif par le LLM
// ----------------------------------------------------------------------------

export const RecapAnswerSchema = z.object({
  players: z.string().min(1),
  gm: z.string().min(1),
});

export function recapMessages(input: {
  campaignName: string;
  story: CampaignStory;
  facts: RecapFacts;
  publicTimeline: string[];
  gmTimeline: string[];
  previousGmRecap?: string;
}): LlmMessage[] {
  const f = input.facts;
  const system = `Tu es le co-MJ d’une campagne de jeu de rôle, en français. Tu rédiges les récapitulatifs de fin de session.
Réponds UNIQUEMENT avec un objet JSON {"players": "…", "gm": "…"} :
- "players" : « Précédemment… » lu aux joueurs au début de la prochaine session. 4 à 8 phrases au passé, à la deuxième personne du pluriel, vivantes. Uniquement ce que les joueurs ont vécu ou appris (journal public, indices trouvés, révélations connues). AUCUN secret, aucune information réservée au MJ, aucun nom d’adversaire qu’ils ne connaissent pas.
- "gm" : résumé pour le MJ, en puces : ce qui s’est passé, fils ouverts et indices manquants, menaces qui avancent, conséquences à prévoir, 2-3 pistes pour la prochaine session.`;
  const facts = [
    `Scènes jouées : ${f.scenesVisited.map((s) => `${s.title}${s.resolved ? " (résolue)" : ""}`).join(", ") || "aucune"}`,
    `Indices trouvés : ${f.cluesFound.map((c) => c.text).join(" | ") || "aucun"}`,
    `Révélations désormais connues des joueurs : ${f.revelationsLearned.map((r) => r.statement).join(" | ") || "aucune"}`,
    `Menaces (MJ) : ${f.frontsAdvanced.map((x) => `${x.name} → ${x.step}`).join(" | ") || "aucune n’a avancé"}`,
    `Adversaires vaincus : ${f.fallen.map((x) => `${x.name} (connu des joueurs comme « ${x.visibleName} »)`).join(", ") || "aucun"}`,
    `Groupe : ${f.party.map((p) => `${p.name} ${p.down ? "à terre" : p.hp !== null ? `${p.hp}/${p.hpMax} PV` : ""}`).join(", ")}`,
  ].join("\n");
  const missing = input.story.revelations
    .filter((r) => r.importance === "critical" && !f.revelationsLearned.some((x) => x.id === r.id))
    .map((r) => r.statement);
  const user = `# Campagne « ${input.campaignName} »
Pitch : ${input.story.bible.pitch}
Ton : ${input.story.bible.tone}
${input.previousGmRecap ? `\n# Session précédente (MJ)\n${input.previousGmRecap}\n` : ""}
# Faits de la session
${facts}
Révélations essentielles encore inconnues (MJ) : ${missing.join(" | ") || "aucune"}

# Journal public (ce que les joueurs ont vu)
${input.publicTimeline.slice(-80).map((t) => `- ${t}`).join("\n") || "- (vide)"}

# Journal MJ complet
${input.gmTimeline.slice(-120).map((t) => `- ${t}`).join("\n") || "- (vide)"}`;
  return [
    { role: "system", content: system },
    { role: "user", content: user },
  ];
}
