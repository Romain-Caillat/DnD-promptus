import { describe, it, expect } from "vitest";
import { DND5E_RULESET } from "@/lib/engine/ruleset";
import { parseJsonObject } from "@/lib/ai/llm";
import { BudgetExceededError, generateCampaign } from "@/lib/generation/pipeline";
import { GenerationInputSchema } from "@/lib/generation/types";
import { FakeLlm } from "../../scripts/fake-llm";

const input = GenerationInputSchema.parse({ pitch: "Des gobelins pillent un village minier." });

describe("parseJsonObject", () => {
  it("extrait un objet d’un bloc de code ou de texte bavard", () => {
    expect(parseJsonObject('Voici :\n```json\n{"a":1}\n```')).toEqual({ a: 1 });
    expect(parseJsonObject('Bien sûr ! {"b":[1,2]} Bonne partie.')).toEqual({ b: [1, 2] });
  });
  it("échoue clairement sans objet JSON", () => {
    expect(() => parseJsonObject("désolé")).toThrow(/objet JSON/);
  });
});

describe("generateCampaign", () => {
  it("produit un brouillon valide en trois étapes + vérification", async () => {
    const llm = new FakeLlm();
    const steps: string[] = [];
    const r = await generateCampaign(input, {
      llm,
      ruleset: DND5E_RULESET,
      onStep: (id, status) => {
        steps.push(`${id}:${status}`);
      },
    });
    expect(llm.calls).toEqual(["cast", "scenes", "maps"]);
    expect(r.issues).toEqual([]);
    expect(r.draft.story.scenes).toHaveLength(7);
    expect(r.draft.story.scenes.find((s) => s.id === "sc_camp")?.battleMapId).toBe("map_camp");
    expect(r.draft.entities.every((e) => e.ref.startsWith("ent_"))).toBe(true);
    expect(r.usage.costUsd).toBeCloseTo(0.03);
    expect(steps).toEqual([
      "cast:running", "cast:done", "scenes:running", "scenes:done",
      "maps:running", "maps:done", "check:running", "check:done",
    ]);
  });

  it("renvoie les défauts au LLM et garde sa correction", async () => {
    const llm = new FakeLlm({ breakClues: true });
    const r = await generateCampaign(input, { llm, ruleset: DND5E_RULESET });
    expect(llm.calls).toEqual(["cast", "scenes", "maps", "repair"]);
    expect(r.issues).toEqual([]);
    expect(r.draft.story.clues).toHaveLength(8);
  });

  it("redemande quand le JSON est invalide", async () => {
    const llm = new FakeLlm({ badJsonFirst: true });
    const r = await generateCampaign(input, { llm, ruleset: DND5E_RULESET });
    expect(llm.calls.slice(0, 2)).toEqual(["cast-bad", "retry"]);
    expect(r.issues).toEqual([]);
  });

  it("s’arrête quand le budget est dépassé", async () => {
    const llm = new FakeLlm({ costPerCall: 0.5 });
    await expect(generateCampaign(input, { llm, ruleset: DND5E_RULESET, budgetUsd: 1 })).rejects.toBeInstanceOf(
      BudgetExceededError,
    );
    expect(llm.calls).toEqual(["cast", "scenes", "maps"]);
  });
});
