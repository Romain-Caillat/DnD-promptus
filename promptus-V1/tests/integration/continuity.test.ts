import { describe, it, expect, beforeAll, afterAll } from "vitest";
import { eq } from "drizzle-orm";
import { db } from "@/lib/db/client";
import { campaigns, sessions, sessionTimeline } from "@/lib/db/schema";
import { generateId } from "@/lib/api/ids";
import { normalizeWorld } from "@/lib/engine/world";
import { endSession, generateRecapWithLlm, previousRecaps, updateRecap } from "@/lib/continuity/server";
import { buildDemoStory } from "../../scripts/demo-story";
import { FakeLlm } from "../../scripts/fake-llm";

describe("fin de session et récapitulatif", () => {
  const campaignId = generateId("camp");
  const s1 = generateId("ses");
  const s2 = generateId("ses");
  const story = buildDemoStory((n) => `ent_${n}`);
  const clue = story.clues[0];

  beforeAll(async () => {
    await db.insert(campaigns).values({
      id: campaignId,
      name: "Continuité",
      story,
      worldState: { ...normalizeWorld(null), foundClueIds: [clue.id], currentSceneId: clue.sceneId, music: { videoId: "dQw4w9WgXcQ", startedAt: 0, offsetSec: 0, playing: true } },
    });
    await db.insert(sessions).values({ id: s1, campaignId, name: "Session 1", startedAt: new Date(Date.now() - 3_600_000), worldAtStart: normalizeWorld(null) });
    await db.insert(sessionTimeline).values({ id: generateId("tl"), sessionId: s1, description: "Les héros arrivent" });
    await db.insert(sessions).values({ id: s2, campaignId, name: "Session 2" });
  });

  afterAll(async () => {
    await db.delete(campaigns).where(eq(campaigns.id, campaignId));
  });

  it("mesure la session, coupe la musique et prépare un brouillon factuel", async () => {
    const s = await endSession(s1);
    expect(s.endedAt).toBeTruthy();
    expect(s.recap?.status).toBe("draft");
    expect(s.recap?.facts.cluesFound.map((c) => c.id)).toEqual([clue.id]);
    expect(s.recap?.players).toContain(clue.text);
    const [c] = await db.select().from(campaigns).where(eq(campaigns.id, campaignId));
    expect(normalizeWorld(c.worldState).music).toBeUndefined();
    await expect(endSession(s1)).rejects.toThrow("déjà terminée");
  });

  it("réécrit avec l’IA, puis le MJ publie", async () => {
    await generateRecapWithLlm(s1, new FakeLlm());
    let [row] = await db.select().from(sessions).where(eq(sessions.id, s1));
    expect(row.recap?.source).toBe("llm");
    expect(row.recap?.players).toContain("Précédemment");
    row = await updateRecap(s1, { players: "Précédemment, version du MJ.", publish: true });
    expect(row.recap?.status).toBe("published");
    expect(row.recap?.players).toBe("Précédemment, version du MJ.");
  });

  it("la session suivante retrouve le récapitulatif", async () => {
    const [next] = await db.select().from(sessions).where(eq(sessions.id, s2));
    const prev = await previousRecaps(campaignId, next.startedAt, 2);
    expect(prev.map((p) => p.id)).toEqual([s1]);
  });
});
