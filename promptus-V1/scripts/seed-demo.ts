import { config } from "dotenv";
config({ path: ".env.local" });

import { db } from "../src/lib/db/client";
import { campaigns, entities } from "../src/lib/db/schema";
import { generateId } from "../src/lib/api/ids";
import { eq, like } from "drizzle-orm";
import type { Effect } from "../src/lib/engine/types";
import type { NewEntityRow } from "../src/lib/db/schema";

/**
 * Builds the demo campaign "The Goblin Dungeon" with enough content to play
 * a one-shot session of ~3 hours at level 1.
 *
 * Idempotent: removes any prior campaign whose name starts with "The Goblin
 * Dungeon — Demo" before re-creating.
 *
 * Runs independently of seed-audio.ts; both can be combined in `pnpm seed:all`.
 */

async function main() {
  console.log("[seed-demo] starting...");

  // Clean previous demo campaigns (cascades to entities)
  const prev = await db
    .select()
    .from(campaigns)
    .where(like(campaigns.name, "The Goblin Dungeon — Demo%"));
  for (const c of prev) {
    await db.delete(campaigns).where(eq(campaigns.id, c.id));
    console.log(`[seed-demo] removed previous campaign ${c.id}`);
  }

  const campaignId = generateId("camp");
  await db.insert(campaigns).values({
    id: campaignId,
    name: "The Goblin Dungeon — Demo",
    description:
      "A 3-hour one-shot at level 1 — gateway scenario for first-time GMs. Four heroes, a haunted crypt, a goblin warband, and a forgotten king who refuses to die.",
    styleGuide: {
      artStyle: "dark fantasy oil painting, painterly",
      mood: "ominous, candle-lit, dramatic",
      palette: "muted earth tones with deep blacks",
      promptPrefix: "dark fantasy, dramatic lighting,",
      promptSuffix: "detailed, cinematic composition",
      negativePrompt: "modern, neon, cartoon, low quality, deformed",
    },
    systemTemplate: "dnd5e",
  });
  console.log(`[seed-demo] campaign created: ${campaignId}`);

  const ents: NewEntityRow[] = [];

  // -----------------------------------------------------------------------
  // PLAYER CHARACTERS (4)
  // -----------------------------------------------------------------------
  ents.push({
    id: generateId("ent_character"),
    campaignId,
    type: "character",
    name: "Bobby the Brave",
    description: "A young human fighter from a frontier village; fierce, protective, prone to overconfidence.",
    tags: ["fighter", "human", "lvl 1", "PC"],
    attributes: {
      hp: 12,
      hpMax: 12,
      ac: 16,
      speed: 30,
      level: 1,
      classes: [{ name: "Fighter", level: 1 }],
      abilityScores: { STR: 16, DEX: 13, CON: 14, INT: 10, WIS: 11, CHA: 8 },
      proficiencyBonus: 2,
      initiativeBonus: 1,
    },
    effects: [],
    visibility: "public",
    version: 1,
  });

  ents.push({
    id: generateId("ent_character"),
    campaignId,
    type: "character",
    name: "Léa Stormcaller",
    description: "An elven wizard with a staff carved from ancient yew; curious, patient, hides a ruthless streak.",
    tags: ["wizard", "elf", "lvl 1", "PC"],
    attributes: {
      hp: 7,
      hpMax: 7,
      ac: 12,
      speed: 30,
      level: 1,
      classes: [{ name: "Wizard", level: 1 }],
      abilityScores: { STR: 8, DEX: 14, CON: 12, INT: 16, WIS: 13, CHA: 10 },
      proficiencyBonus: 2,
      initiativeBonus: 2,
      spellSlots: { 1: { current: 2, max: 2 } },
    },
    effects: [],
    visibility: "public",
    version: 1,
  });

  ents.push({
    id: generateId("ent_character"),
    campaignId,
    type: "character",
    name: "Tom Quickfingers",
    description: "A halfling rogue with too many daggers and not enough morals; sneaky, strategic, secretly loyal.",
    tags: ["rogue", "halfling", "lvl 1", "PC"],
    attributes: {
      hp: 9,
      hpMax: 9,
      ac: 14,
      speed: 25,
      level: 1,
      classes: [{ name: "Rogue", level: 1 }],
      abilityScores: { STR: 9, DEX: 17, CON: 13, INT: 12, WIS: 11, CHA: 14 },
      proficiencyBonus: 2,
      initiativeBonus: 3,
    },
    effects: [],
    visibility: "public",
    version: 1,
  });

  ents.push({
    id: generateId("ent_character"),
    campaignId,
    type: "character",
    name: "Anaïs Lightbringer",
    description: "A human cleric of a forgotten dawn god; gentle in counsel, fierce in battle, rarely lets the party rest enough.",
    tags: ["cleric", "human", "lvl 1", "PC"],
    attributes: {
      hp: 10,
      hpMax: 10,
      ac: 18,
      speed: 25,
      level: 1,
      classes: [{ name: "Cleric", level: 1 }],
      abilityScores: { STR: 12, DEX: 10, CON: 14, INT: 11, WIS: 16, CHA: 13 },
      proficiencyBonus: 2,
      initiativeBonus: 0,
      spellSlots: { 1: { current: 2, max: 2 } },
    },
    effects: [],
    visibility: "public",
    version: 1,
  });

  // -----------------------------------------------------------------------
  // SPELLS (15 essentials)
  // -----------------------------------------------------------------------
  function spell(
    name: string,
    description: string,
    level: number,
    school: string,
    effects: Effect[],
    extra: Record<string, unknown> = {},
  ): NewEntityRow {
    return {
      id: generateId("ent_spell"),
      campaignId,
      type: "spell",
      name,
      description,
      tags: [school, `level ${level}`],
      attributes: {
        level,
        school,
        castingTime: "1 action",
        range: "30 ft",
        components: ["V", "S"],
        duration: "instantaneous",
        ...extra,
      },
      effects,
      visibility: "public",
      version: 1,
    };
  }

  ents.push(
    spell(
      "Magic Missile",
      "Three darts of force unerringly strike chosen targets.",
      1,
      "evocation",
      [
        { type: "consume_resource", resource: "spell_slot", amount: 1, level: 1, target: { type: "caster" } },
        { type: "damage", amount: "3d4+3", damageType: "force", target: { type: "single", entityId: "" } },
      ],
      { range: "120 ft" },
    ),
    spell(
      "Fireball",
      "An explosion of flame detonates at a chosen point.",
      3,
      "evocation",
      [
        { type: "consume_resource", resource: "spell_slot", amount: 1, level: 3, target: { type: "caster" } },
        {
          type: "roll_check",
          stat: "DEX",
          dc: 15,
          target: { type: "all_in_area" },
          outcomeFail: [
            { type: "damage", amount: "8d6", damageType: "fire", target: { type: "all_in_area" } },
          ],
          outcomeSuccess: [
            { type: "damage", amount: "8d6/2", damageType: "fire", target: { type: "all_in_area" } },
          ],
        },
        { type: "play_sound", soundId: "Spell sparkle" },
      ],
      { range: "150 ft", components: ["V", "S", "M"] },
    ),
    spell(
      "Cure Wounds",
      "A creature you touch regains hit points.",
      1,
      "evocation",
      [
        { type: "consume_resource", resource: "spell_slot", amount: 1, level: 1, target: { type: "caster" } },
        { type: "heal", amount: "1d8+3", target: { type: "single", entityId: "" } },
      ],
      { range: "touch" },
    ),
    spell(
      "Sleep",
      "A magical slumber takes the weakest creatures in an area.",
      1,
      "enchantment",
      [
        { type: "consume_resource", resource: "spell_slot", amount: 1, level: 1, target: { type: "caster" } },
        {
          type: "apply_condition",
          conditionId: "unconscious",
          duration: { minutes: 1 },
          target: { type: "all_in_area" },
        },
      ],
      { range: "90 ft" },
    ),
    spell(
      "Shield",
      "An invisible barrier of force protects you.",
      1,
      "abjuration",
      [
        { type: "consume_resource", resource: "spell_slot", amount: 1, level: 1, target: { type: "caster" } },
        { type: "modify_stat", stat: "AC", modifier: 5, duration: { rounds: 1 }, target: { type: "self" } },
      ],
      { castingTime: "1 reaction" },
    ),
    spell(
      "Bless",
      "Allies gain a bonus to attacks and saves.",
      1,
      "enchantment",
      [
        { type: "consume_resource", resource: "spell_slot", amount: 1, level: 1, target: { type: "caster" } },
        { type: "modify_stat", stat: "attack_bonus", modifier: 4, duration: { minutes: 1 }, target: { type: "all_players" } },
      ],
      { range: "30 ft" },
    ),
    spell(
      "Sacred Flame",
      "Radiant fire descends on a target.",
      0,
      "evocation",
      [
        {
          type: "roll_check",
          stat: "DEX",
          dc: 13,
          target: { type: "single", entityId: "" },
          outcomeFail: [
            { type: "damage", amount: "1d8", damageType: "radiant", target: { type: "single", entityId: "" } },
          ],
        },
      ],
      { range: "60 ft" },
    ),
    spell(
      "Fire Bolt",
      "A mote of fire streaks toward a target.",
      0,
      "evocation",
      [
        { type: "damage", amount: "1d10", damageType: "fire", target: { type: "single", entityId: "" } },
      ],
      { range: "120 ft" },
    ),
    spell(
      "Eldritch Blast",
      "A beam of crackling energy.",
      0,
      "evocation",
      [
        { type: "damage", amount: "1d10", damageType: "force", target: { type: "single", entityId: "" } },
      ],
      { range: "120 ft" },
    ),
    spell(
      "Shocking Grasp",
      "Lightning springs from your hand to deliver a shock.",
      0,
      "evocation",
      [
        { type: "damage", amount: "1d8", damageType: "lightning", target: { type: "single", entityId: "" } },
      ],
      { range: "touch" },
    ),
    spell(
      "Hold Person",
      "Choose a humanoid you can see; it must succeed on a Wisdom save or be paralyzed.",
      2,
      "enchantment",
      [
        { type: "consume_resource", resource: "spell_slot", amount: 1, level: 2, target: { type: "caster" } },
        {
          type: "roll_check",
          stat: "WIS",
          dc: 13,
          target: { type: "single", entityId: "" },
          outcomeFail: [
            {
              type: "apply_condition",
              conditionId: "paralyzed",
              duration: { minutes: 1 },
              target: { type: "single", entityId: "" },
            },
          ],
        },
      ],
      { range: "60 ft" },
    ),
    spell(
      "Misty Step",
      "Briefly surrounded by silvery mist, you teleport up to 30 feet.",
      2,
      "conjuration",
      [
        { type: "consume_resource", resource: "spell_slot", amount: 1, level: 2, target: { type: "caster" } },
        { type: "display_text", text: "A wisp of silver mist marks your departure." },
      ],
    ),
    spell(
      "Mage Armor",
      "A protective magical force surrounds the target.",
      1,
      "abjuration",
      [
        { type: "consume_resource", resource: "spell_slot", amount: 1, level: 1, target: { type: "caster" } },
        { type: "modify_stat", stat: "AC", modifier: 3, duration: { minutes: 480 }, target: { type: "single", entityId: "" } },
      ],
    ),
    spell(
      "Light",
      "An object you touch sheds bright light.",
      0,
      "evocation",
      [{ type: "display_text", text: "The object shines brightly." }],
    ),
    spell(
      "Burning Hands",
      "A thin sheet of flames shoots from your fingertips.",
      1,
      "evocation",
      [
        { type: "consume_resource", resource: "spell_slot", amount: 1, level: 1, target: { type: "caster" } },
        {
          type: "roll_check",
          stat: "DEX",
          dc: 13,
          target: { type: "all_in_area" },
          outcomeFail: [
            { type: "damage", amount: "3d6", damageType: "fire", target: { type: "all_in_area" } },
          ],
          outcomeSuccess: [
            { type: "damage", amount: "3d6/2", damageType: "fire", target: { type: "all_in_area" } },
          ],
        },
      ],
      { range: "15 ft cone" },
    ),
  );

  // -----------------------------------------------------------------------
  // MONSTERS (10)
  // -----------------------------------------------------------------------
  function monster(
    name: string,
    description: string,
    cr: number,
    hp: number,
    ac: number,
    abilities: Record<string, number>,
    extraTags: string[] = [],
  ): NewEntityRow {
    return {
      id: generateId("ent_monster"),
      campaignId,
      type: "monster",
      name,
      description,
      tags: ["monster", `CR ${cr}`, ...extraTags],
      attributes: {
        hp,
        hpMax: hp,
        ac,
        speed: 30,
        challengeRating: cr,
        size: "Medium",
        abilityScores: { STR: 10, DEX: 10, CON: 10, INT: 10, WIS: 10, CHA: 10, ...abilities },
        initiativeBonus: Math.floor(((abilities.DEX ?? 10) - 10) / 2),
      },
      effects: [],
      visibility: "mj_only",
      version: 1,
    };
  }

  ents.push(
    monster("Goblin Scout", "A small, sneaky humanoid lurking in shadows.", 0.25, 7, 15, { DEX: 14 }, ["goblin", "humanoid"]),
    monster("Goblin Warrior", "A bigger goblin armed with a notched scimitar.", 0.25, 10, 14, { STR: 12, DEX: 13 }, ["goblin", "humanoid"]),
    monster("Goblin Boss", "Cunning leader of the goblin band; never fights fair.", 1, 21, 17, { STR: 10, DEX: 14, CHA: 10 }, ["goblin", "humanoid", "leader"]),
    monster("Giant Rat", "Bigger than any honest rat has a right to be.", 0.125, 7, 12, { DEX: 11 }, ["beast"]),
    monster("Giant Spider", "Eight legs, eight eyes, far too much patience.", 1, 26, 14, { DEX: 16 }, ["beast", "spider"]),
    monster("Skeleton", "Bones held together by malice.", 0.25, 13, 13, { STR: 10, DEX: 14 }, ["undead"]),
    monster("Zombie", "Slow, relentless, hungry.", 0.25, 22, 8, { CON: 16 }, ["undead"]),
    monster("Bandit", "A masked human looking for easy coin.", 0.125, 11, 12, { STR: 11, DEX: 12 }, ["humanoid", "criminal"]),
    monster("Wolf", "A pack hunter with a mean bite.", 0.25, 11, 13, { DEX: 15 }, ["beast"]),
    monster("Revenant Mort-Roi", "The skeletal remains of the forgotten king refuse to rest.", 5, 65, 17, { STR: 18, DEX: 14, CON: 16, WIS: 16 }, ["undead", "boss", "named"]),
  );

  // -----------------------------------------------------------------------
  // ITEMS (10)
  // -----------------------------------------------------------------------
  function item(name: string, description: string, rarity: "common" | "uncommon" | "rare", tags: string[]): NewEntityRow {
    return {
      id: generateId("ent_item"),
      campaignId,
      type: "item",
      name,
      description,
      tags,
      attributes: { rarity, weight: 3, value: 15 },
      effects: [],
      visibility: "public",
      version: 1,
    };
  }

  ents.push(
    item("Longsword", "A versatile straight-bladed sword.", "common", ["weapon", "martial", "melee"]),
    item("Shortbow", "Quick and light.", "common", ["weapon", "ranged"]),
    item("Dagger", "Small, fast, easily concealed.", "common", ["weapon", "finesse"]),
    item("Healing Potion", "A small vial of crimson liquid.", "uncommon", ["potion", "healing", "consumable"]),
    item("Chain Shirt", "Light metal links over a leather backing.", "common", ["armor", "medium"]),
    item("Torch", "Burns for an hour, casts dim light.", "common", ["light", "consumable"]),
    item("Rope (50 ft)", "Hempen, sturdy.", "common", ["utility"]),
    item("Spellbook", "Contains the spells of an apprentice mage.", "common", ["book", "magic", "wizard"]),
    item("Holy Symbol", "Etched in silver, a sun with rays.", "common", ["focus", "cleric"]),
    item("Goblin Banner", "A faded, ragged banner showing a snarling goblin.", "uncommon", ["loot", "trophy", "named"]),
  );

  // -----------------------------------------------------------------------
  // LOCATIONS (5)
  // -----------------------------------------------------------------------
  function location(name: string, description: string, ambienceName: string, tags: string[]): NewEntityRow {
    return {
      id: generateId("ent_location"),
      campaignId,
      type: "location",
      name,
      description,
      tags,
      attributes: {
        defaultAmbienceName: ambienceName, // resolved by tag-matching in EntityPreview
      },
      effects: [],
      visibility: "public",
      version: 1,
    };
  }

  ents.push(
    location("Tin Hollow village", "A damp mining village; cobbled streets, tin smoke in the air.", "Tavern bustle", ["tavern", "village", "town"]),
    location("The Yew Inn", "Wooden tables, a fire that never quite warms, suspicious patrons.", "Tavern bustle", ["tavern", "indoor"]),
    location("The Forgotten Crypt", "A damp stone sanctuary buried beneath an old chapel.", "Crypt drips", ["crypt", "dungeon", "underground"]),
    location("The Whispering Woods", "Old oaks and a stillness that prickles the neck.", "Forest at night", ["forest", "outdoor", "night"]),
    location("Castle Aldric", "Crumbling battlements, banners of a dead house.", "Castle hall", ["castle", "indoor", "noble"]),
  );

  // -----------------------------------------------------------------------
  // NPCs (4)
  // -----------------------------------------------------------------------
  function npc(name: string, description: string, faction: string, motivation: string, secret?: string): NewEntityRow {
    return {
      id: generateId("ent_npc"),
      campaignId,
      type: "npc",
      name,
      description,
      tags: [faction.toLowerCase().replace(/\s+/g, "-")],
      attributes: {
        hp: 12,
        hpMax: 12,
        ac: 12,
        faction,
        status: "alive",
        motivation,
        secret,
      },
      effects: [],
      visibility: "mj_only",
      version: 1,
    };
  }

  ents.push(
    npc(
      "Bortrand the Robust",
      "Mayor of Tin Hollow; weary but determined.",
      "Tin Hollow Council",
      "Protect his village from goblin raids.",
      "His son was taken by goblins three weeks ago.",
    ),
    npc(
      "Sister Mira",
      "Cleric of the dawn at the village chapel.",
      "Order of the Dawning Sun",
      "Find what desecrates the crypts beneath her chapel.",
      "Suspects the mayor's son became a revenant.",
    ),
    npc(
      "Old Tomas",
      "The blacksmith. Knows everyone, owes a few favors, won't say which.",
      "Tin Hollow Council",
      "Keep the village's secrets.",
    ),
    npc(
      "Greta the Brewer",
      "Owner of the Yew Inn. Loud, hospitable, tracks gossip like prey.",
      "Yew Inn",
      "Keep her customers happy and her information flowing.",
    ),
  );

  // Insert all
  for (const e of ents) {
    await db.insert(entities).values(e);
  }
  console.log(`[seed-demo] inserted ${ents.length} entities`);

  console.log(`\n[seed-demo] DONE`);
  console.log(`  campaignId: ${campaignId}`);
  console.log(`  open:       /campaigns/${campaignId}`);
  console.log(`  entities:   /campaigns/${campaignId}/entities`);
  process.exit(0);
}

main().catch((err) => {
  console.error("[seed-demo] failed:", err);
  process.exit(1);
});
