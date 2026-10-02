import { describe, it, expect } from "vitest";
import { normalizeWorld } from "@/lib/engine/world";
import { parseJsonObject } from "@/lib/ai/llm";
import { RecapAnswerSchema, factsRecap, recapMessages, sessionFacts, type FactsInput } from "@/lib/continuity/recap";
import { buildDemoStory } from "../../scripts/demo-story";
import { FakeLlm } from "../../scripts/fake-llm";

const story = buildDemoStory((n) => `e:${n}`);
const crypt = story.scenes.find((s) => s.id === "sc_crypte")!;
const clue = story.clues.find((c) => c.sceneId === "sc_crypte")!;
const front = story.fronts[0];

function input(over: Partial<FactsInput> = {}): FactsInput {
  const before = { ...normalizeWorld(null), currentSceneId: "sc_auberge", sceneStatus: { sc_auberge: "visited" as const } };
  const after = {
    ...normalizeWorld(null),
    currentSceneId: "sc_crypte",
    sceneStatus: { sc_auberge: "resolved" as const, sc_crypte: "visited" as const },
    foundClueIds: [clue.id],
    frontProgress: { [front.id]: 1 },
    revealedEntityIds: ["e:Sœur Mira"],
  };
  return {
    story,
    before,
    after,
    entities: [
      { id: "e:Bobby", name: "Bobby", type: "character", visibility: "public" },
      { id: "e:Squelette", name: "Squelette", type: "monster", visibility: "mj_only" },
      { id: "e:Sœur Mira", name: "Sœur Mira", type: "npc", visibility: "mj_only" },
    ],
    states: [
      { entityId: "e:Bobby", state: { hp: 4, hpMax: 12, conditions: [] } },
      { entityId: "e:Squelette", state: { hp: 0, hpMax: 13, conditions: [] } },
    ],
    timeline: ["⚔️ Le combat commence ! Initiative lancée.", "Bobby attaque Squelette", "🏁 Fin de la session"],
    startedAt: new Date("2026-10-02T18:00:00Z"),
    endedAt: new Date("2026-10-02T21:30:00Z"),
    ...over,
  };
}

describe("sessionFacts", () => {
  it("mesure ce que la session a changé", () => {
    const f = sessionFacts(input());
    expect(f.durationMinutes).toBe(210);
    expect(f.scenesVisited.map((s) => [s.id, s.resolved])).toEqual(
      expect.arrayContaining([["sc_auberge", true], ["sc_crypte", false]]),
    );
    expect(f.cluesFound.map((c) => c.id)).toEqual([clue.id]);
    expect(f.revelationsLearned.map((r) => r.id)).toEqual([clue.revelationId]);
    expect(f.frontsAdvanced).toEqual([expect.objectContaining({ id: front.id, from: -1, to: 1, step: front.steps[1].label })]);
    expect(f.entitiesRevealed).toEqual(["Sœur Mira"]);
    expect(f.combats).toBe(1);
    expect(f.fallen).toEqual([{ name: "Squelette", visibleName: "un adversaire" }]);
    expect(f.party).toEqual([{ name: "Bobby", hp: 4, hpMax: 12, down: false }]);
  });

  it("ne compte que les nouveautés", () => {
    const i = input();
    const f = sessionFacts({ ...i, before: { ...i.after } });
    expect(f.cluesFound).toEqual([]);
    expect(f.frontsAdvanced).toEqual([]);
    expect(f.entitiesRevealed).toEqual([]);
  });
});

describe("factsRecap", () => {
  it("le récap joueurs ne contient ni menace ni nom caché", () => {
    const f = sessionFacts(input());
    const { gm, players } = factsRecap(f);
    expect(gm).toContain(front.name);
    expect(gm).toContain("Squelette");
    expect(players).toContain(crypt.title);
    expect(players).toContain(clue.text);
    expect(players).not.toContain(front.name);
    expect(players).not.toContain("Squelette");
    expect(players).toContain("un adversaire");
  });
});

describe("recap par le LLM", () => {
  it("transmet faits et journaux, et lit la réponse", async () => {
    const f = sessionFacts(input());
    const messages = recapMessages({
      campaignName: "Démo",
      story,
      facts: f,
      publicTimeline: ["Bobby attaque Adversaire"],
      gmTimeline: ["Bobby attaque Squelette"],
    });
    expect(messages[1].content).toContain("Journal public");
    expect(messages[1].content).toContain(clue.text);
    const res = await new FakeLlm().complete({ messages, json: true });
    const answer = RecapAnswerSchema.parse(parseJsonObject(res.text));
    expect(answer.players).toContain("Précédemment");
    expect(answer.gm).toContain(crypt.title);
  });
});
