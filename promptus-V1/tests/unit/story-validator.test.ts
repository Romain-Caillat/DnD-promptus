import { describe, it, expect } from "vitest";
import { DND5E_RULESET } from "@/lib/engine/ruleset";
import { EMPTY_STORY, type CampaignStory, type Scene } from "@/lib/engine/story";
import { validateStory } from "@/lib/engine/story-validator";
import { StorySchema } from "@/lib/validation/story-schema";
import { buildDemoStory } from "../../scripts/demo-story";

const ctx = (ids: string[] = []) => ({ entityIds: new Set(ids), ruleset: DND5E_RULESET });

function scene(id: string, exits: string[] = [], extra: Partial<Scene> = {}): Scene {
  return {
    id,
    title: id,
    summary: "",
    objective: "",
    readAloud: "Texte.",
    phase: "exploration",
    npcEntityIds: [],
    monsterEntityIds: [],
    exits: exits.map((toSceneId) => ({ toSceneId, label: "" })),
    triggers: [],
    ...extra,
  };
}

function story(partial: Partial<CampaignStory>): CampaignStory {
  return {
    ...EMPTY_STORY,
    ...partial,
    bible: { ...EMPTY_STORY.bible, pitch: "Pitch.", ...partial.bible },
  };
}

const messages = (s: CampaignStory, ids?: string[]) => validateStory(s, ctx(ids));

describe("validateStory", () => {
  it("valide la démo sans erreur ni avertissement", () => {
    const demo = buildDemoStory((name) => `ent:${name}`);
    const allIds = JSON.stringify(demo).match(/ent:[^"]+/g) ?? [];
    expect(StorySchema.safeParse(demo).success).toBe(true);
    expect(messages(demo, allIds)).toEqual([]);
  });

  it("signale les références cassées comme erreurs", () => {
    const s = story({
      bible: { ...EMPTY_STORY.bible, startSceneId: "a" },
      scenes: [scene("a", ["zzz"], { npcEntityIds: ["ent_inconnu"] })],
    });
    const issues = messages(s);
    expect(issues.filter((i) => i.severity === "error").map((i) => i.path)).toEqual([
      "scenes[0].npcEntityIds[0]",
      "scenes[0].exits[0].toSceneId",
    ]);
  });

  it("applique la règle des trois indices", () => {
    const s = story({
      bible: { ...EMPTY_STORY.bible, startSceneId: "a" },
      scenes: [scene("a", ["b"]), scene("b")],
      revelations: [{ id: "r", statement: "Le maire ment", importance: "critical" }],
      clues: [
        { id: "c1", revelationId: "r", sceneId: "a", text: "x", discovery: "" },
        { id: "c2", revelationId: "r", sceneId: "a", text: "y", discovery: "" },
      ],
    });
    const warnings = messages(s).filter((i) => i.severity === "warning").map((i) => i.message);
    expect(warnings.some((m) => m.includes("Règle des trois indices"))).toBe(true);
    expect(warnings.some((m) => m.includes("même scène"))).toBe(true);
  });

  it("repère les scènes inaccessibles et les impasses multiples", () => {
    const s = story({
      bible: { ...EMPTY_STORY.bible, startSceneId: "a" },
      scenes: [scene("a", ["b"]), scene("b"), scene("orpheline")],
    });
    const warnings = messages(s).map((i) => i.message);
    expect(warnings.some((m) => m.includes("« orpheline » est inaccessible"))).toBe(true);
    expect(warnings.some((m) => m.includes("2 scènes sans sortie"))).toBe(true);
  });

  it("vérifie grilles, niveaux de cartes et effets contre les règles", () => {
    const s = story({
      maps: [
        { id: "m1", name: "Monde", level: "campaign", grid: { type: "square", cols: 4, rows: 4 }, cells: [{ x: 9, y: 0, childMapId: "m2" }] },
        { id: "m2", name: "Crypte", level: "local", grid: { type: "square", cols: 4, rows: 4 }, cells: [] },
      ],
      fronts: [
        {
          id: "f",
          name: "Menace",
          goal: "",
          description: "",
          steps: [{ label: "1", description: "", effects: [{ type: "apply_condition", conditionId: "maudit", target: { type: "self" } }] }],
        },
      ],
    });
    const errors = messages(s).filter((i) => i.severity === "error").map((i) => i.message);
    expect(errors).toEqual(
      expect.arrayContaining([
        expect.stringContaining("grille hexagonale"),
        expect.stringContaining("hors de « Monde »"),
        expect.stringContaining("ne peut pas s’ouvrir depuis une carte campagne"),
        expect.stringContaining("État absent des règles : « maudit »"),
      ]),
    );
  });
});
