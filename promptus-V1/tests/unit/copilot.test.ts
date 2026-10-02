import { describe, it, expect } from "vitest";
import { DND5E_RULESET } from "@/lib/engine/ruleset";
import { normalizeWorld } from "@/lib/engine/world";
import { parseJsonObject } from "@/lib/ai/llm";
import { buildCopilotContext, copilotMessages, sanitizeAnswer, type CopilotContextInput } from "@/lib/copilot/copilot";
import { buildDemoStory } from "../../scripts/demo-story";
import { FakeLlm } from "../../scripts/fake-llm";

const story = buildDemoStory((n) => `ent_${n.toLowerCase().replace(/[^a-z]+/g, "_")}`);
const ids = new Set<string>();
for (const s of story.scenes) for (const id of [...s.npcEntityIds, ...s.monsterEntityIds, ...(s.locationEntityId ? [s.locationEntityId] : [])]) ids.add(id);
const entities = [...ids].map((id) => ({
  id,
  name: id.replace("ent_", ""),
  type: (story.scenes.some((s) => s.npcEntityIds.includes(id)) ? "npc" : "monster") as never,
  description: "desc",
  attributes: { motivation: "survivre", secret: "il ment" },
}));

function input(over: Partial<CopilotContextInput> = {}): CopilotContextInput {
  return {
    campaignName: "Démo",
    story,
    world: { ...normalizeWorld(null), currentSceneId: "sc_crypte" },
    ruleset: DND5E_RULESET,
    entities,
    session: { phase: "exploration", combatRound: 0, activeTurnIndex: 0, initiativeOrder: [] },
    participants: [],
    timeline: ["Bobby fouille le sarcophage", "🎲 Bobby — Investigation DD 12 → échec"],
    pendingRequests: ["Alice : Observer (« le couloir »)"],
    ...over,
  };
}

describe("buildCopilotContext", () => {
  it("contient la scène en cours, ses indices, ses sorties, les menaces et le journal", () => {
    const ctx = buildCopilotContext(input());
    const scene = story.scenes.find((s) => s.id === "sc_crypte")!;
    expect(ctx).toContain(`sc_crypte « ${scene.title} »`);
    expect(ctx).toContain(scene.summary);
    for (const c of story.clues.filter((c) => c.sceneId === "sc_crypte")) expect(ctx).toContain(c.id);
    for (const x of scene.exits) expect(ctx).toContain(x.toSceneId);
    for (const f of story.fronts) expect(ctx).toContain(f.id);
    expect(ctx).toContain("Bobby fouille le sarcophage");
    expect(ctx).toContain("Alice : Observer");
  });

  it("reste compact", () => {
    expect(buildCopilotContext(input()).length).toBeLessThan(12_000);
  });
});

describe("sanitizeAnswer", () => {
  it("garde les actions valides et retire celles aux identifiants inconnus", () => {
    const clue = story.clues[0];
    const a = sanitizeAnswer(
      {
        narration: "  Le vent se lève.  ",
        npcLines: [{ npcId: entities[0].id, text: "Bonjour" }, { text: "   " }],
        suggestions: [
          { label: "Indice", action: { type: "reveal_clue", clueId: clue.id } },
          { label: "Inventé", action: { type: "reveal_clue", clueId: "cl_nope" } },
          { label: "Bizarre", action: { type: "delete_everything" } },
        ],
      },
      story,
      entities,
    );
    expect(a.narration).toBe("Le vent se lève.");
    expect(a.npcLines).toEqual([{ npcId: entities[0].id, speaker: entities[0].name, text: "Bonjour" }]);
    expect(a.suggestions.map((s) => !!s.action)).toEqual([true, false, false]);
  });

  it("refuse une réponse illisible", () => {
    expect(() => sanitizeAnswer({ narration: 42 }, story, entities)).toThrow();
  });
});

describe("co-MJ avec le faux modèle", () => {
  it("produit une réponse exploitable à partir du contexte", async () => {
    const llm = new FakeLlm();
    const res = await llm.complete({ messages: copilotMessages({ kind: "consequence", prompt: "ils crient" }, input()), json: true });
    const a = sanitizeAnswer(parseJsonObject(res.text), story, entities);
    expect(a.narration).toContain("ils crient");
    expect(a.suggestions.filter((s) => s.action).length).toBeGreaterThanOrEqual(2);
    expect(a.suggestions.some((s) => s.label === "Action fantaisiste" && !s.action)).toBe(true);
  });
});
