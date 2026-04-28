import { describe, it, expect, beforeAll, afterAll } from "vitest";
import { db } from "@/lib/db/client";
import { campaigns, entities } from "@/lib/db/schema";
import { eq } from "drizzle-orm";
import { generateId } from "@/lib/api/ids";

describe("CRUD entities", () => {
  let campaignId: string;
  let entityId: string;

  beforeAll(async () => {
    campaignId = generateId("camp");
    await db.insert(campaigns).values({
      id: campaignId,
      name: "Test Campaign",
    });
  });

  afterAll(async () => {
    await db.delete(entities).where(eq(entities.campaignId, campaignId));
    await db.delete(campaigns).where(eq(campaigns.id, campaignId));
  });

  it("creates a spell entity", async () => {
    entityId = generateId("ent_spell");
    const [row] = await db
      .insert(entities)
      .values({
        id: entityId,
        campaignId,
        type: "spell",
        name: "Fireball",
        description: "A fiery explosion",
        tags: ["evocation", "fire"],
        attributes: {
          level: 3,
          school: "evocation",
          castingTime: "1 action",
          range: "150 ft",
          components: ["V", "S", "M"],
          duration: "instantaneous",
        },
        effects: [
          {
            type: "consume_resource",
            resource: "spell_slot",
            amount: 1,
            level: 3,
            target: { type: "caster" },
          },
          {
            type: "roll_check",
            stat: "DEX",
            dc: 14,
            target: { type: "all_in_area" },
            outcomeFail: [
              {
                type: "damage",
                amount: "8d6",
                damageType: "fire",
                target: { type: "all_in_area" },
              },
            ],
          },
        ],
        visibility: "public",
      })
      .returning();
    expect(row.name).toBe("Fireball");
    expect(row.effects).toHaveLength(2);
  });

  it("reads the spell entity", async () => {
    const [row] = await db.select().from(entities).where(eq(entities.id, entityId));
    expect(row).toBeDefined();
    expect(row.type).toBe("spell");
  });

  it("updates the spell entity", async () => {
    const [row] = await db
      .update(entities)
      .set({ description: "An updated description", updatedAt: new Date() })
      .where(eq(entities.id, entityId))
      .returning();
    expect(row.description).toBe("An updated description");
  });

  it("deletes the spell entity", async () => {
    const [row] = await db
      .delete(entities)
      .where(eq(entities.id, entityId))
      .returning();
    expect(row.id).toBe(entityId);
  });
});
