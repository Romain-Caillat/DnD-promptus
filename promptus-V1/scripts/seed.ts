import { config } from "dotenv";
config({ path: ".env.local" });

import { db } from "../src/lib/db/client";
import { campaigns, entities } from "../src/lib/db/schema";
import { generateId } from "../src/lib/api/ids";
import type { Effect } from "../src/lib/engine/types";
import type { NewEntityRow } from "../src/lib/db/schema";

async function main() {
  console.log("[seed] starting...");

  const campaignId = generateId("camp");
  await db.insert(campaigns).values({
    id: campaignId,
    name: "The Goblin Dungeon — Demo",
    description:
      "A classic 3-hour one-shot at level 1. Used as a tutorial scenario for first-time GMs.",
    styleGuide: {
      artStyle: "dark fantasy, oil painting style",
      mood: "ominous, candle-lit",
      promptPrefix: "dark fantasy, dramatic lighting,",
      promptSuffix: "muted palette, painterly",
    },
    systemTemplate: "dnd5e",
  });
  console.log(`[seed] campaign created: ${campaignId}`);

  const fireballEffects: Effect[] = [
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
      dc: 15,
      target: { type: "all_in_area" },
      outcomeFail: [
        {
          type: "damage",
          amount: "8d6",
          damageType: "fire",
          target: { type: "all_in_area" },
        },
      ],
      outcomeSuccess: [
        {
          type: "damage",
          amount: "8d6/2",
          damageType: "fire",
          target: { type: "all_in_area" },
        },
      ],
    },
  ];

  const seedEntities: NewEntityRow[] = [
    {
      id: generateId("ent_spell"),
      campaignId,
      type: "spell" as const,
      name: "Fireball",
      description:
        "An explosion of fire detonates at a chosen point, scorching everything within 20 ft.",
      tags: ["evocation", "fire", "AoE", "level 3"],
      attributes: {
        level: 3,
        school: "evocation",
        castingTime: "1 action",
        range: "150 ft",
        components: ["V", "S", "M"],
        duration: "instantaneous",
      },
      effects: fireballEffects,
      visibility: "public" as const,
      version: 1,
    },
    {
      id: generateId("ent_monster"),
      campaignId,
      type: "monster" as const,
      name: "Goblin Scout",
      description: "A small, sneaky humanoid lurking in the shadows of the dungeon.",
      tags: ["goblin", "humanoid", "small", "CR 1/4"],
      attributes: {
        hp: 7,
        hpMax: 7,
        ac: 15,
        speed: 30,
        challengeRating: 0.25,
        size: "Small",
        alignment: "Neutral Evil",
        abilityScores: { STR: 8, DEX: 14, CON: 10, INT: 10, WIS: 8, CHA: 8 },
        initiativeBonus: 2,
      },
      effects: [],
      visibility: "mj_only" as const,
      version: 1,
    },
    {
      id: generateId("ent_npc"),
      campaignId,
      type: "npc" as const,
      name: "Bortrand the Robust",
      description:
        "Mayor of a tin-mining village; weary but determined. Looks bigger than he is.",
      tags: ["mayor", "human", "village", "ally"],
      attributes: {
        hp: 12,
        hpMax: 12,
        ac: 12,
        faction: "Tin Hollow Council",
        currentLocation: "Tin Hollow",
        status: "alive",
        motivation: "Protect his village from goblin raids",
        secret: "His son is missing, taken by goblins three weeks ago.",
        voiceActorRecommended: "deep, paternal, slow",
      },
      effects: [],
      visibility: "mj_only" as const,
      version: 1,
    },
    {
      id: generateId("ent_item"),
      campaignId,
      type: "item" as const,
      name: "Longsword",
      description: "A versatile sword with a straight, double-edged blade.",
      tags: ["weapon", "martial", "melee"],
      attributes: {
        rarity: "common",
        weight: 3,
        value: 15,
      },
      effects: [
        {
          type: "damage" as const,
          amount: "1d8",
          damageType: "slashing" as const,
          target: { type: "single" as const, entityId: "" },
        },
      ] as Effect[],
      visibility: "public" as const,
      version: 1,
    },
    {
      id: generateId("ent_location"),
      campaignId,
      type: "location" as const,
      name: "The Forgotten Crypt",
      description:
        "A damp stone sanctuary buried beneath an old chapel; cold air seeps from within.",
      tags: ["dungeon", "underground", "ancient", "spooky"],
      attributes: {
        defaultAmbienceId: "ambience_crypt",
      },
      effects: [],
      visibility: "public" as const,
      version: 1,
    },
  ];

  for (const e of seedEntities) {
    await db.insert(entities).values(e);
    console.log(`[seed] entity created: ${e.type} — ${e.name}`);
  }

  console.log(`[seed] done. campaignId=${campaignId}`);
  process.exit(0);
}

main().catch((err) => {
  console.error("[seed] failed:", err);
  process.exit(1);
});
