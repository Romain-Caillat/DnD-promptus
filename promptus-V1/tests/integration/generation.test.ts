import { describe, it, expect, beforeAll, afterAll } from "vitest";
import { eq } from "drizzle-orm";
import { db } from "@/lib/db/client";
import { campaigns, entities } from "@/lib/db/schema";
import { generateId } from "@/lib/api/ids";
import { GenerationInputSchema } from "@/lib/generation/types";
import { applyDraft, getJob, runGenerationJob, startGenerationJob, toJobView } from "@/lib/generation/server";
import { validateStory } from "@/lib/engine/story-validator";
import { DND5E_RULESET } from "@/lib/engine/ruleset";
import { FakeLlm } from "../../scripts/fake-llm";

describe("génération de campagne (base de données)", () => {
  let campaignId: string;

  beforeAll(async () => {
    campaignId = generateId("camp");
    await db.insert(campaigns).values({ id: campaignId, name: "Test génération", aiSettings: { budgetUsd: 1 } });
  });

  afterAll(async () => {
    await db.delete(campaigns).where(eq(campaigns.id, campaignId));
  });

  it("exécute un job, puis applique le brouillon en créant les fiches", async () => {
    const input = GenerationInputSchema.parse({ pitch: "Des gobelins pillent un village minier." });
    const job = await startGenerationJob(campaignId, input);
    await runGenerationJob(job.id, new FakeLlm({ breakClues: true }));

    const done = toJobView(await getJob(job.id));
    expect(done.status).toBe("succeeded");
    expect(done.steps.every((s) => s.status === "done")).toBe(true);
    expect(done.result?.issues).toEqual([]);
    expect(done.usage.costUsd).toBeCloseTo(0.04);

    const { entities: created } = await applyDraft(job.id);
    const ents = await db.select().from(entities).where(eq(entities.campaignId, campaignId));
    expect(ents).toHaveLength(created);
    const [campaign] = await db.select().from(campaigns).where(eq(campaigns.id, campaignId));
    const story = campaign.story!;
    const json = JSON.stringify(story);
    for (const e of done.result!.draft.entities) expect(json).not.toContain(`"${e.ref}"`); // références remplacées
    expect(validateStory(story, { entityIds: new Set(ents.map((e) => e.id)), ruleset: DND5E_RULESET })).toEqual([]);
    expect(toJobView(await getJob(job.id)).status).toBe("applied");
    await expect(applyDraft(job.id)).rejects.toThrow(/déjà été appliqué/);
  });

  it("refuse de démarrer quand le budget est épuisé", async () => {
    await db.update(campaigns).set({ aiSettings: { budgetUsd: 0.01 } }).where(eq(campaigns.id, campaignId));
    const input = GenerationInputSchema.parse({ pitch: "Une autre idée de campagne." });
    await expect(startGenerationJob(campaignId, input)).rejects.toThrow(/Budget de génération épuisé/);
  });
});
