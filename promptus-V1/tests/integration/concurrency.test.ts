import { describe, it, expect, beforeAll, afterAll } from "vitest";
import { eq } from "drizzle-orm";
import { NextRequest } from "next/server";
import { db } from "@/lib/db/client";
import { campaigns, entities, sessions, sessionState } from "@/lib/db/schema";
import { generateId } from "@/lib/api/ids";
import { normalizeWorld } from "@/lib/engine/world";
import type { EntityState } from "@/lib/engine/types";
import { runAction } from "@/lib/session/run-action";
import { POST as mapPost } from "@/app/api/sessions/[id]/map/route";
import { buildDemoStory } from "../../scripts/demo-story";

// Plusieurs requêtes simultanées (MJ et joueurs) ne doivent pas s'écraser.
describe("écritures concurrentes de l'état du monde", () => {
  const campaignId = generateId("camp");
  const sessionId = generateId("sess");
  const targetId = generateId("ent_monster");

  beforeAll(async () => {
    await db.insert(campaigns).values({ id: campaignId, name: "Concurrence", story: buildDemoStory((n) => `ent_${n}`) });
    await db.insert(entities).values({ id: targetId, campaignId, type: "monster", name: "Cible", attributes: { hp: 100, hpMax: 100 } });
    await db.insert(sessions).values({ id: sessionId, campaignId, name: "S" });
    await db.insert(sessionState).values({
      id: generateId("st"),
      sessionId,
      entityId: targetId,
      currentState: { hp: 100, hpMax: 100, conditions: [] },
    });
  });

  afterAll(async () => {
    await db.delete(campaigns).where(eq(campaigns.id, campaignId));
  });

  it("garde toutes les cases révélées en parallèle", async () => {
    const reveal = (x: number) =>
      mapPost(
        new NextRequest("http://test/api", {
          method: "POST",
          body: JSON.stringify({ op: "reveal", mapId: "map_crypte", cells: [[x, 1]] }),
        }),
        { params: Promise.resolve({ id: sessionId }) },
      );
    const res = await Promise.all(Array.from({ length: 12 }, (_, x) => reveal(x)));
    expect(res.every((r) => r.status === 200)).toBe(true);
    const [c] = await db.select().from(campaigns).where(eq(campaigns.id, campaignId));
    expect(normalizeWorld(c.worldState).mapState.map_crypte.revealed).toHaveLength(12);
  });

  it("cumule les dégâts simultanés sur une même cible", async () => {
    await Promise.all(
      Array.from({ length: 10 }, () =>
        runAction(sessionId, {
          kind: "raw_effects",
          targetIds: [targetId],
          effects: [{ type: "damage", amount: "1", damageType: "fire", target: { type: "single", entityId: targetId } }],
        }),
      ),
    );
    const [st] = await db.select().from(sessionState).where(eq(sessionState.entityId, targetId));
    expect((st.currentState as EntityState).hp).toBe(90);
  });
});
