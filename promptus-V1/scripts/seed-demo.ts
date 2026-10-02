import { config } from "dotenv";
config({ path: ".env.local" });

import { db } from "../src/lib/db/client";
import { campaigns, entities } from "../src/lib/db/schema";
import { generateId } from "../src/lib/api/ids";
import { SCHOOL_LABELS } from "../src/lib/engine/catalog";
import { DND5E_RULESET } from "../src/lib/engine/ruleset";
import { validateStory } from "../src/lib/engine/story-validator";
import { buildDemoStory } from "./demo-story";
import { eq, like } from "drizzle-orm";
import type { Effect } from "../src/lib/engine/types";
import type { NewEntityRow } from "../src/lib/db/schema";

/**
 * Builds the demo campaign "Le Donjon des gobelins" with enough content to play
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
    .where(like(campaigns.name, "%(démo)%"));
  for (const c of prev) {
    await db.delete(campaigns).where(eq(campaigns.id, c.id));
    console.log(`[seed-demo] removed previous campaign ${c.id}`);
  }

  const campaignId = generateId("camp");
  await db.insert(campaigns).values({
    id: campaignId,
    name: "Le Donjon des gobelins (démo)",
    description:
      "Un one-shot de 3 heures au niveau 1, idéal pour un premier MJ. Quatre héros, une crypte hantée, une bande de gobelins et un roi oublié qui refuse de mourir.",
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
    name: "Bobby le Brave",
    description: "Jeune guerrier humain d’un village frontalier ; farouche, protecteur, trop sûr de lui.",
    tags: ["guerrier", "humain", "niv. 1", "PJ"],
    attributes: {
      hp: 12,
      hpMax: 12,
      ac: 16,
      speed: 30,
      level: 1,
      classes: [{ name: "Guerrier", level: 1 }],
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
    name: "Léa Mandetempête",
    description: "Magicienne elfe au bâton taillé dans un if ancien ; curieuse, patiente, cache un côté impitoyable.",
    tags: ["magicien", "elfe", "niv. 1", "PJ"],
    attributes: {
      hp: 7,
      hpMax: 7,
      ac: 12,
      speed: 30,
      level: 1,
      classes: [{ name: "Magicien", level: 1 }],
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
    name: "Tom Doigts-Agiles",
    description: "Roublard halfelin, trop de dagues et pas assez de morale ; furtif, stratège, loyal en secret.",
    tags: ["roublard", "halfelin", "niv. 1", "PJ"],
    attributes: {
      hp: 9,
      hpMax: 9,
      ac: 14,
      speed: 25,
      level: 1,
      classes: [{ name: "Roublard", level: 1 }],
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
    name: "Anaïs Porte-Lumière",
    description: "Clerc humaine d’un dieu de l’aube oublié ; douce en conseil, farouche au combat, laisse rarement le groupe se reposer.",
    tags: ["clerc", "humain", "niv. 1", "PJ"],
    attributes: {
      hp: 10,
      hpMax: 10,
      ac: 18,
      speed: 25,
      level: 1,
      classes: [{ name: "Clerc", level: 1 }],
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
      tags: [SCHOOL_LABELS[school] ?? school, `niveau ${level}`],
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
      "Projectile magique",
      "Trois fléchettes de force frappent les cibles choisies sans jamais manquer.",
      1,
      "evocation",
      [
        { type: "consume_resource", resource: "spell_slot", amount: 1, level: 1, target: { type: "caster" } },
        { type: "damage", amount: "3d4+3", damageType: "force", target: { type: "single", entityId: "" } },
      ],
      { range: "120 ft" },
    ),
    spell(
      "Boule de feu",
      "Une explosion de flammes détone au point choisi.",
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
        { type: "play_sound", soundId: "Scintillement magique" },
      ],
      { range: "45 m", components: ["V", "S", "M"] },
    ),
    spell(
      "Soins",
      "La créature touchée récupère des points de vie.",
      1,
      "evocation",
      [
        { type: "consume_resource", resource: "spell_slot", amount: 1, level: 1, target: { type: "caster" } },
        { type: "heal", amount: "1d8+3", target: { type: "single", entityId: "" } },
      ],
      { range: "touch" },
    ),
    spell(
      "Sommeil",
      "Un sommeil magique emporte les créatures les plus faibles de la zone.",
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
      "Bouclier",
      "Une barrière de force invisible vous protège.",
      1,
      "abjuration",
      [
        { type: "consume_resource", resource: "spell_slot", amount: 1, level: 1, target: { type: "caster" } },
        { type: "modify_stat", stat: "AC", modifier: 5, duration: { rounds: 1 }, target: { type: "self" } },
      ],
      { castingTime: "1 reaction" },
    ),
    spell(
      "Bénédiction",
      "Les alliés gagnent un bonus aux attaques et aux sauvegardes.",
      1,
      "enchantment",
      [
        { type: "consume_resource", resource: "spell_slot", amount: 1, level: 1, target: { type: "caster" } },
        { type: "modify_stat", stat: "attack_bonus", modifier: 4, duration: { minutes: 1 }, target: { type: "all_players" } },
      ],
      { range: "30 ft" },
    ),
    spell(
      "Flamme sacrée",
      "Un feu radieux s’abat sur une cible.",
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
      { range: "18 m" },
    ),
    spell(
      "Trait de feu",
      "Une étincelle de feu file vers une cible.",
      0,
      "evocation",
      [
        { type: "damage", amount: "1d10", damageType: "fire", target: { type: "single", entityId: "" } },
      ],
      { range: "120 ft" },
    ),
    spell(
      "Décharge occulte",
      "Un rayon d’énergie crépitante.",
      0,
      "evocation",
      [
        { type: "damage", amount: "1d10", damageType: "force", target: { type: "single", entityId: "" } },
      ],
      { range: "120 ft" },
    ),
    spell(
      "Poigne électrique",
      "Un éclair jaillit de votre main et électrise la cible.",
      0,
      "evocation",
      [
        { type: "damage", amount: "1d8", damageType: "lightning", target: { type: "single", entityId: "" } },
      ],
      { range: "touch" },
    ),
    spell(
      "Immobilisation de personne",
      "Un humanoïde que vous voyez doit réussir une sauvegarde de Sagesse ou être paralysé.",
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
      { range: "18 m" },
    ),
    spell(
      "Foulée brumeuse",
      "Entouré un instant d’une brume argentée, vous vous téléportez jusqu’à 9 m.",
      2,
      "conjuration",
      [
        { type: "consume_resource", resource: "spell_slot", amount: 1, level: 2, target: { type: "caster" } },
        { type: "display_text", text: "Un filet de brume argentée marque votre départ." },
      ],
    ),
    spell(
      "Armure de mage",
      "Une force magique protectrice entoure la cible.",
      1,
      "abjuration",
      [
        { type: "consume_resource", resource: "spell_slot", amount: 1, level: 1, target: { type: "caster" } },
        { type: "modify_stat", stat: "AC", modifier: 3, duration: { minutes: 480 }, target: { type: "single", entityId: "" } },
      ],
    ),
    spell(
      "Lumière",
      "L’objet touché émet une vive lumière.",
      0,
      "evocation",
      [{ type: "display_text", text: "L’objet brille intensément." }],
    ),
    spell(
      "Mains brûlantes",
      "Une fine nappe de flammes jaillit du bout de vos doigts.",
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
        size: "Moyenne",
        abilityScores: { STR: 10, DEX: 10, CON: 10, INT: 10, WIS: 10, CHA: 10, ...abilities },
        initiativeBonus: Math.floor(((abilities.DEX ?? 10) - 10) / 2),
      },
      effects: [],
      visibility: "mj_only",
      version: 1,
    };
  }

  ents.push(
    monster("Éclaireur gobelin", "Petit humanoïde sournois tapi dans l’ombre.", 0.25, 7, 15, { DEX: 14 }, ["gobelin", "humanoïde"]),
    monster("Guerrier gobelin", "Un gobelin plus costaud, armé d’un cimeterre ébréché.", 0.25, 10, 14, { STR: 12, DEX: 13 }, ["gobelin", "humanoïde"]),
    monster("Chef gobelin", "Chef rusé de la bande ; ne se bat jamais à la loyale.", 1, 21, 17, { STR: 10, DEX: 14, CHA: 10 }, ["gobelin", "humanoïde", "chef"]),
    monster("Rat géant", "Plus gros qu’un honnête rat n’a le droit de l’être.", 0.125, 7, 12, { DEX: 11 }, ["bête"]),
    monster("Araignée géante", "Huit pattes, huit yeux, et bien trop de patience.", 1, 26, 14, { DEX: 16 }, ["bête", "araignée"]),
    monster("Squelette", "Des os que seule la malveillance tient ensemble.", 0.25, 13, 13, { STR: 10, DEX: 14 }, ["mort-vivant"]),
    monster("Zombie", "Lent, implacable, affamé.", 0.25, 22, 8, { CON: 16 }, ["mort-vivant"]),
    monster("Bandit", "Un humain masqué en quête d’argent facile.", 0.125, 11, 12, { STR: 11, DEX: 12 }, ["humanoïde", "criminel"]),
    monster("Loup", "Chasseur de meute à la morsure cruelle.", 0.25, 11, 13, { DEX: 15 }, ["bête"]),
    monster("Revenant Mort-Roi", "La dépouille squelettique du roi oublié refuse le repos.", 5, 65, 17, { STR: 18, DEX: 14, CON: 16, WIS: 16 }, ["mort-vivant", "boss", "unique"]),
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
    item("Épée longue", "Une épée droite et polyvalente.", "common", ["arme", "de guerre", "corps à corps"]),
    item("Arc court", "Rapide et léger.", "common", ["arme", "distance"]),
    item("Dague", "Petite, rapide, facile à dissimuler.", "common", ["arme", "finesse"]),
    item("Potion de soins", "Une petite fiole de liquide écarlate.", "uncommon", ["potion", "soin", "consommable"]),
    item("Chemise de mailles", "De fins anneaux de métal sur un dos de cuir.", "common", ["armure", "intermédiaire"]),
    item("Torche", "Brûle une heure et diffuse une faible lumière.", "common", ["lumière", "consommable"]),
    item("Corde (15 m)", "En chanvre, solide.", "common", ["utilitaire"]),
    item("Grimoire", "Contient les sorts d’un apprenti mage.", "common", ["livre", "magie", "magicien"]),
    item("Symbole sacré", "Un soleil rayonnant gravé dans l’argent.", "common", ["focaliseur", "clerc"]),
    item("Bannière gobeline", "Une bannière délavée et déchirée montrant un gobelin grimaçant.", "uncommon", ["butin", "trophée", "unique"]),
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
        defaultAmbienceName: ambienceName, // résolu par nom dans EntityPreview
      },
      effects: [],
      visibility: "public",
      version: 1,
    };
  }

  ents.push(
    location("Village de Creux-d’Étain", "Village minier humide ; rues pavées, fumée d’étain dans l’air.", "Brouhaha de taverne", ["taverne", "village", "ville"]),
    location("L’Auberge de l’If", "Tables de bois, un feu qui ne réchauffe jamais vraiment, clients louches.", "Brouhaha de taverne", ["taverne", "intérieur"]),
    location("La Crypte oubliée", "Un sanctuaire de pierre humide enfoui sous une vieille chapelle.", "Gouttes dans la crypte", ["crypte", "donjon", "souterrain"]),
    location("Le Bois des Murmures", "De vieux chênes et un silence qui hérisse la nuque.", "Forêt de nuit", ["forêt", "extérieur", "nuit"]),
    location("Château d’Aldric", "Remparts en ruine, bannières d’une maison éteinte.", "Grand hall de château", ["château", "intérieur", "noble"]),
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
      "Bortrand le Robuste",
      "Bourgmestre de Creux-d’Étain ; épuisé mais déterminé.",
      "Conseil de Creux-d’Étain",
      "Protéger son village des raids gobelins.",
      "Son fils a été enlevé par les gobelins il y a trois semaines.",
    ),
    npc(
      "Sœur Mira",
      "Prêtresse de l’aube à la chapelle du village.",
      "Ordre du Soleil levant",
      "Découvrir ce qui profane les cryptes sous sa chapelle.",
      "Soupçonne le fils du bourgmestre d’être devenu un revenant.",
    ),
    npc(
      "Le vieux Tomas",
      "Le forgeron. Connaît tout le monde, doit quelques faveurs, refuse de dire lesquelles.",
      "Conseil de Creux-d’Étain",
      "Garder les secrets du village.",
    ),
    npc(
      "Greta la Brasseuse",
      "Tenancière de l’Auberge de l’If. Bruyante, accueillante, traque les ragots comme du gibier.",
      "Auberge de l’If",
      "Garder ses clients contents et les informations qui circulent.",
    ),
  );

  // Insert all
  for (const e of ents) {
    await db.insert(entities).values(e);
  }
  console.log(`[seed-demo] inserted ${ents.length} entities`);

  // Histoire V2 : bible, fronts, scènes, indices, cartes.
  const idByName = new Map(ents.map((e) => [e.name, e.id]));
  const story = buildDemoStory((name) => {
    const id = idByName.get(name);
    if (!id) throw new Error(`[seed-demo] fiche introuvable : ${name}`);
    return id;
  });
  const issues = validateStory(story, {
    entityIds: new Set(ents.map((e) => e.id)),
    ruleset: DND5E_RULESET,
  });
  for (const i of issues) console.log(`[seed-demo] ${i.severity} ${i.path} — ${i.message}`);
  if (issues.some((i) => i.severity === "error")) throw new Error("[seed-demo] histoire invalide");
  await db.update(campaigns).set({ story }).where(eq(campaigns.id, campaignId));
  console.log(`[seed-demo] story: ${story.scenes.length} scènes, ${story.clues.length} indices, ${story.maps.length} cartes`);

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
