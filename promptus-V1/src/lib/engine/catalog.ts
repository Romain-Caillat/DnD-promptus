// Static catalogs used by the entity editor and runtime.
// These are strings shown in dropdowns; engine logic does not depend on them
// being these specific values — it only needs the IDs you assign.

import type {
  DamageType,
  EntityType,
  ResourceKind,
  Stat,
  Visibility,
} from "./types";

export const ENTITY_TYPES: { id: EntityType; label: string }[] = [
  { id: "spell", label: "Spell" },
  { id: "item", label: "Item" },
  { id: "npc", label: "NPC" },
  { id: "monster", label: "Monster" },
  { id: "character", label: "Character" },
  { id: "location", label: "Location" },
  { id: "event", label: "Event" },
  { id: "condition", label: "Condition" },
  { id: "faction", label: "Faction" },
];

export const VISIBILITIES: { id: Visibility; label: string }[] = [
  { id: "public", label: "Public (everyone)" },
  { id: "mj_only", label: "GM only" },
  { id: "players_in_session", label: "Players in session" },
  { id: "specific_users", label: "Specific users" },
];

export const STATS: Stat[] = ["STR", "DEX", "CON", "INT", "WIS", "CHA"];

export const DAMAGE_TYPES: DamageType[] = [
  "slashing",
  "piercing",
  "bludgeoning",
  "fire",
  "cold",
  "lightning",
  "thunder",
  "acid",
  "poison",
  "psychic",
  "necrotic",
  "radiant",
  "force",
];

export const RESOURCE_KINDS: ResourceKind[] = [
  "spell_slot",
  "ki",
  "rage",
  "sorcery_point",
  "bardic_inspiration",
  "channel_divinity",
  "action_surge",
  "second_wind",
  "custom",
];

export const SCHOOLS_OF_MAGIC = [
  "abjuration",
  "conjuration",
  "divination",
  "enchantment",
  "evocation",
  "illusion",
  "necromancy",
  "transmutation",
];

export const STANDARD_CONDITIONS = [
  "blinded",
  "charmed",
  "deafened",
  "frightened",
  "grappled",
  "incapacitated",
  "invisible",
  "paralyzed",
  "petrified",
  "poisoned",
  "prone",
  "restrained",
  "stunned",
  "unconscious",
] as const;

export type StandardConditionId = (typeof STANDARD_CONDITIONS)[number];

// 20 primitive effect kinds + label + category
export const EFFECT_KINDS = [
  // Combat
  { id: "damage", label: "Inflict damage", category: "combat" },
  { id: "heal", label: "Heal", category: "combat" },
  { id: "apply_condition", label: "Apply condition", category: "combat" },
  { id: "remove_condition", label: "Remove condition", category: "combat" },
  { id: "modify_stat", label: "Modify stat", category: "combat" },
  { id: "roll_check", label: "Roll check / saving throw", category: "combat" },
  { id: "consume_resource", label: "Consume resource", category: "combat" },
  { id: "restore_resource", label: "Restore resource", category: "combat" },
  // Narration
  { id: "set_state", label: "Set entity state", category: "narration" },
  { id: "move_entity", label: "Move entity to location", category: "narration" },
  { id: "reveal_entity", label: "Reveal entity", category: "narration" },
  { id: "set_relation", label: "Adjust relation", category: "narration" },
  { id: "trigger_event", label: "Trigger event", category: "narration" },
  { id: "add_to_inventory", label: "Add to inventory", category: "narration" },
  { id: "remove_from_inventory", label: "Remove from inventory", category: "narration" },
  // Sensory
  { id: "play_ambience", label: "Play ambience", category: "sensory" },
  { id: "play_music", label: "Play music", category: "sensory" },
  { id: "play_sound", label: "Play sound effect", category: "sensory" },
  { id: "display_image", label: "Display image", category: "sensory" },
  { id: "display_text", label: "Display text", category: "sensory" },
] as const;

export type EffectKindId = (typeof EFFECT_KINDS)[number]["id"];

export const TARGET_KINDS = [
  { id: "self", label: "Self (the activator)" },
  { id: "caster", label: "Caster" },
  { id: "single", label: "A single target" },
  { id: "multiple", label: "Several targets" },
  { id: "all_in_area", label: "All in area" },
  { id: "all_players", label: "All players" },
  { id: "all_enemies", label: "All enemies" },
] as const;
