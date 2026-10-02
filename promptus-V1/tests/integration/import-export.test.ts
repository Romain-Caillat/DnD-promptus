import { describe, it, expect, beforeAll, afterAll } from "vitest";
import * as yaml from "js-yaml";
import { db } from "@/lib/db/client";
import { campaigns, entities } from "@/lib/db/schema";
import { eq } from "drizzle-orm";
import { generateId } from "@/lib/api/ids";
import { EntityInputSchema } from "@/lib/validation/entity-schemas";

/**
 * These tests exercise the YAML round-trip logic against the real schema —
 * but without going through the HTTP layer (which requires a running server).
 * They verify that the schemas accept and reject what the routes will see.
 */

describe("YAML import/export round-trip", () => {
  let campaignId: string;

  beforeAll(async () => {
    campaignId = generateId("camp");
    await db.insert(campaigns).values({
      id: campaignId,
      name: "Import Test Campaign",
    });
  });

  afterAll(async () => {
    await db.delete(entities).where(eq(entities.campaignId, campaignId));
    await db.delete(campaigns).where(eq(campaigns.id, campaignId));
  });

  it("parses a single-document YAML entity", () => {
    const yamlText = `
type: spell
name: Magic Missile
description: Three darts of force.
tags: [evocation, force]
attributes:
  level: 1
  school: evocation
  castingTime: 1 action
  range: 120 ft
  components: [V, S]
  duration: instantaneous
effects:
  - type: damage
    amount: 3d4+3
    damageType: force
    target:
      type: single
      entityId: "ent_x"
visibility: public
`;
    const parsed = yaml.load(yamlText);
    const result = EntityInputSchema.omit({ campaignId: true }).safeParse(parsed);
    expect(result.success).toBe(true);
    if (result.success) {
      expect(result.data.name).toBe("Magic Missile");
      expect(result.data.effects).toHaveLength(1);
    }
  });

  it("rejects a YAML with invalid effect type", () => {
    const yamlText = `
type: spell
name: Broken
attributes: {}
effects:
  - type: not_a_real_effect
    amount: 1d6
visibility: public
`;
    const parsed = yaml.load(yamlText);
    const result = EntityInputSchema.omit({ campaignId: true }).safeParse(parsed);
    expect(result.success).toBe(false);
  });

  it("parses a multi-document YAML stream", () => {
    const yamlText = `
type: item
name: Sword
attributes: { rarity: common }
effects: []
visibility: public
---
type: monster
name: Goblin
attributes: { hp: 7, hpMax: 7, ac: 15 }
effects: []
visibility: mj_only
`;
    const docs = yaml.loadAll(yamlText).filter((d) => d != null);
    expect(docs).toHaveLength(2);
    for (const doc of docs) {
      const result = EntityInputSchema.omit({ campaignId: true }).safeParse(doc);
      expect(result.success).toBe(true);
    }
  });

  it("round-trips an entity through DB and dumps back to YAML", async () => {
    const id = generateId("ent_spell");
    await db.insert(entities).values({
      id,
      campaignId,
      type: "spell",
      name: "Round-Trip Spell",
      description: "Test",
      tags: ["test"],
      attributes: { level: 1 },
      effects: [
        {
          type: "damage",
          amount: "1d4",
          damageType: "fire",
          target: { type: "self" },
        },
      ],
      visibility: "public",
      version: 1,
    });
    const [row] = await db.select().from(entities).where(eq(entities.id, id));
    const exported = {
      id: row.id,
      type: row.type,
      name: row.name,
      tags: row.tags,
      attributes: row.attributes,
      effects: row.effects,
      visibility: row.visibility,
    };
    const yamlText = yaml.dump(exported, { lineWidth: 120 });
    expect(yamlText).toContain("Round-Trip Spell");
    expect(yamlText).toContain("type: damage");
    expect(yamlText).toContain("damageType: fire");
  });
});
