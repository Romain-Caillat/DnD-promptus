// Faux LLM déterministe : rejoue la campagne de démo au format attendu par
// le pipeline. Sert aux tests et au faux serveur OpenRouter local.

import type { LlmClient, LlmRequest, LlmResponse } from "../src/lib/ai/llm";
import type { DraftEntity } from "../src/lib/generation/types";
import { buildDemoStory } from "./demo-story";

const slug = (name: string) =>
  "ent_" +
  name
    .normalize("NFD")
    .replace(/[̀-ͯ]/g, "")
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "_")
    .replace(/^_|_$/g, "");

const TYPES: Record<string, DraftEntity["type"]> = {
  "L’Auberge de l’If": "location",
  "Village de Creux-d’Étain": "location",
  "Le Bois des Murmures": "location",
  "La Crypte oubliée": "location",
};

export function demoDraft() {
  const names = new Set<string>();
  const story = buildDemoStory((name) => {
    names.add(name);
    return slug(name);
  });
  const entities: DraftEntity[] = [...names].map((name) => {
    const type = TYPES[name] ?? (story.scenes.some((s) => s.npcEntityIds.includes(slug(name))) ? "npc" : "monster");
    return {
      ref: slug(name),
      type,
      name,
      description: `${name} (démo)`,
      tags: [],
      attributes: type === "monster" ? { hp: 10, hpMax: 10, ac: 12, abilityScores: { DEX: 12 } } : {},
    };
  });
  return { story, entities };
}

export interface FakeLlmOptions {
  /** Étape 2 : ne renvoie que 2 indices pour « Aldo », pour forcer une correction. */
  breakClues?: boolean;
  /** Étape 1 : première réponse en JSON invalide. */
  badJsonFirst?: boolean;
  /** Coût simulé par appel (dollars). */
  costPerCall?: number;
}

export class FakeLlm implements LlmClient {
  readonly name = "fake";
  calls: string[] = [];
  private badJsonPending: boolean;

  constructor(private opts: FakeLlmOptions = {}) {
    this.badJsonPending = !!opts.badJsonFirst;
  }

  async complete(req: LlmRequest): Promise<LlmResponse> {
    const prompt = req.messages.filter((m) => m.role === "user").at(-1)?.content ?? "";
    const { story, entities } = demoDraft();
    let body: unknown;
    let kind: string;
    if (req.messages[0]?.content.includes("Tu rédiges les récapitulatifs")) {
      this.calls.push("recap");
      const ctx = req.messages[1]?.content ?? "";
      const scenes = ctx.match(/^Scènes jouées : (.+)$/m)?.[1] ?? "aucune";
      const clues = ctx.match(/^Indices trouvés : (.+)$/m)?.[1] ?? "aucun";
      return this.reply(
        JSON.stringify({
          players: `Précédemment… Vous avez traversé ${scenes}. Vous avez appris : ${clues}.`,
          gm: `- Scènes : ${scenes}\n- Indices : ${clues}\n- Piste : relancer la menace principale.`,
        }),
        req,
      );
    }
    if (req.messages[0]?.content.includes("Tu aides le MJ humain en direct")) {
      this.calls.push("copilot");
      return this.reply(JSON.stringify(copilotBody(req.messages[1]?.content ?? "")), req);
    }
    if (prompt.includes("ne respecte pas le format")) {
      kind = "retry";
      body = this.castBody(story, entities);
    } else if (prompt.includes("Étape 1/3")) {
      kind = "cast";
      if (this.badJsonPending) {
        this.badJsonPending = false;
        this.calls.push("cast-bad");
        return this.reply("Voici la campagne : { pas du json", req);
      }
      body = this.castBody(story, entities);
    } else if (prompt.includes("Étape 2/3")) {
      kind = "scenes";
      const clues = this.opts.breakClues
        ? story.clues.filter((c) => c.id !== "cl_prisonnier")
        : story.clues;
      body = {
        startSceneId: story.bible.startSceneId,
        scenes: story.scenes.map((s) => {
          const copy = { ...s };
          delete copy.mapPlacement;
          delete copy.battleMapId;
          return copy;
        }),
        revelations: story.revelations,
        clues,
      };
    } else if (prompt.includes("Étape 3/3")) {
      kind = "maps";
      body = {
        maps: story.maps,
        placements: story.scenes.filter((s) => s.mapPlacement).map((s) => ({ sceneId: s.id, ...s.mapPlacement! })),
        battleMaps: story.scenes.filter((s) => s.battleMapId).map((s) => ({ sceneId: s.id, mapId: s.battleMapId! })),
      };
    } else if (prompt.includes("Le validateur a trouvé")) {
      kind = "repair";
      body = story;
    } else {
      throw new Error(`FakeLlm : prompt inattendu : ${prompt.slice(0, 80)}`);
    }
    this.calls.push(kind);
    return this.reply(JSON.stringify(body), req);
  }

  private castBody(story: ReturnType<typeof demoDraft>["story"], entities: DraftEntity[]) {
    const bible = { ...story.bible };
    delete bible.startSceneId;
    return { bible, fronts: story.fronts, entities };
  }

  private reply(text: string, req: LlmRequest): LlmResponse {
    return {
      text,
      model: req.model ?? "fake/demo",
      usage: { promptTokens: 1000, completionTokens: 2000, costUsd: this.opts.costPerCall ?? 0.01 },
    };
  }
}

/** Réponse de co-MJ plausible, construite à partir des identifiants du contexte. */
function copilotBody(context: string) {
  const first = (re: RegExp) => context.match(re)?.[1];
  const npc = first(/^- (ent_\w+) [^\n]*\((?:npc|monster)\)/m);
  const clue = first(/^- (cl_\w+) \(à trouver\)/m);
  const trigger = context.match(/^- (sc_\w+)\/(\w+) « [^»]+ » \(prêt\)/m);
  const exit = first(/^Sorties :\n- (sc_\w+)/m);
  const front = first(/^- (fr_\w+) /m);
  const ask = first(/Précision du MJ : (.+)$/m) ?? first(/Question du MJ : (.+)$/m);
  const suggestions = [
    clue && { label: "Laisser trouver un indice", why: "Les joueurs piétinent", action: { type: "reveal_clue", clueId: clue } },
    trigger && { label: "Déclencher l’événement prêt", action: { type: "fire_trigger", sceneId: trigger[1], triggerId: trigger[2] } },
    front && { label: "La menace progresse", action: { type: "advance_front", frontId: front } },
    exit && { label: "Proposer la scène suivante", action: { type: "enter_scene", sceneId: exit } },
    { label: "Action fantaisiste", action: { type: "reveal_clue", clueId: "cl_inexistant" } },
  ].filter(Boolean);
  return {
    narration: `Une odeur de pierre humide vous saisit. ${ask ? `(${ask}) ` : ""}Au loin, quelque chose racle le sol.`,
    npcLines: npc ? [{ npcId: npc, text: "Vous n’auriez pas dû venir ici." }] : [],
    suggestions,
    gmNote: "Faux co-MJ : réponse de démonstration.",
  };
}
