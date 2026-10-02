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
  { id: "spell", label: "Sort" },
  { id: "item", label: "Objet" },
  { id: "npc", label: "PNJ" },
  { id: "monster", label: "Monstre" },
  { id: "character", label: "Personnage" },
  { id: "location", label: "Lieu" },
  { id: "event", label: "Événement" },
  { id: "condition", label: "État" },
  { id: "faction", label: "Faction" },
];

export const VISIBILITIES: { id: Visibility; label: string }[] = [
  { id: "public", label: "Public (tout le monde)" },
  { id: "mj_only", label: "MJ uniquement" },
  { id: "players_in_session", label: "Joueurs de la session" },
  { id: "specific_users", label: "Utilisateurs choisis" },
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
  { id: "damage", label: "Infliger des dégâts", category: "combat" },
  { id: "heal", label: "Soigner", category: "combat" },
  { id: "apply_condition", label: "Appliquer un état", category: "combat" },
  { id: "remove_condition", label: "Retirer un état", category: "combat" },
  { id: "modify_stat", label: "Modifier une statistique", category: "combat" },
  { id: "roll_check", label: "Jet de caractéristique / sauvegarde", category: "combat" },
  { id: "consume_resource", label: "Consommer une ressource", category: "combat" },
  { id: "restore_resource", label: "Restaurer une ressource", category: "combat" },
  // Narration
  { id: "set_state", label: "Modifier l’état d’une fiche", category: "narration" },
  { id: "move_entity", label: "Déplacer vers un lieu", category: "narration" },
  { id: "reveal_entity", label: "Révéler une fiche", category: "narration" },
  { id: "set_relation", label: "Ajuster une relation", category: "narration" },
  { id: "trigger_event", label: "Déclencher un événement", category: "narration" },
  { id: "add_to_inventory", label: "Ajouter à l’inventaire", category: "narration" },
  { id: "remove_from_inventory", label: "Retirer de l’inventaire", category: "narration" },
  // Sensory
  { id: "play_ambience", label: "Jouer une ambiance", category: "sensory" },
  { id: "play_music", label: "Jouer une musique", category: "sensory" },
  { id: "play_sound", label: "Jouer un bruitage", category: "sensory" },
  { id: "display_image", label: "Afficher une image", category: "sensory" },
  { id: "display_text", label: "Afficher un texte", category: "sensory" },
] as const;

export type EffectKindId = (typeof EFFECT_KINDS)[number]["id"];

export const TARGET_KINDS = [
  { id: "self", label: "Soi-même (l’activateur)" },
  { id: "caster", label: "Lanceur" },
  { id: "single", label: "Une cible" },
  { id: "multiple", label: "Plusieurs cibles" },
  { id: "all_in_area", label: "Tous dans la zone" },
  { id: "all_players", label: "Tous les joueurs" },
  { id: "all_enemies", label: "Tous les ennemis" },
] as const;

// ----------------------------------------------------------------------------
// Libellés français des identifiants internes (les IDs restent en anglais).
// ----------------------------------------------------------------------------

export const ENTITY_TYPE_LABELS: Record<EntityType, string> = Object.fromEntries(
  ENTITY_TYPES.map((t) => [t.id, t.label]),
) as Record<EntityType, string>;

export const STAT_LABELS: Record<Stat, string> = {
  STR: "FOR",
  DEX: "DEX",
  CON: "CON",
  INT: "INT",
  WIS: "SAG",
  CHA: "CHA",
};

export const DAMAGE_TYPE_LABELS: Record<DamageType, string> = {
  slashing: "tranchant",
  piercing: "perforant",
  bludgeoning: "contondant",
  fire: "feu",
  cold: "froid",
  lightning: "foudre",
  thunder: "tonnerre",
  acid: "acide",
  poison: "poison",
  psychic: "psychique",
  necrotic: "nécrotique",
  radiant: "radiant",
  force: "force",
};

export const RESOURCE_LABELS: Record<ResourceKind, string> = {
  spell_slot: "emplacement de sort",
  ki: "ki",
  rage: "rage",
  sorcery_point: "point de sorcellerie",
  bardic_inspiration: "inspiration bardique",
  channel_divinity: "conduit divin",
  action_surge: "fougue",
  second_wind: "second souffle",
  custom: "personnalisée",
};

export const SCHOOL_LABELS: Record<string, string> = {
  abjuration: "abjuration",
  conjuration: "invocation",
  divination: "divination",
  enchantment: "enchantement",
  evocation: "évocation",
  illusion: "illusion",
  necromancy: "nécromancie",
  transmutation: "transmutation",
};

export const CONDITION_LABELS: Record<StandardConditionId, string> = {
  blinded: "aveuglé",
  charmed: "charmé",
  deafened: "assourdi",
  frightened: "effrayé",
  grappled: "agrippé",
  incapacitated: "neutralisé",
  invisible: "invisible",
  paralyzed: "paralysé",
  petrified: "pétrifié",
  poisoned: "empoisonné",
  prone: "à terre",
  restrained: "entravé",
  stunned: "étourdi",
  unconscious: "inconscient",
};

export type PhaseId = "exploration" | "combat" | "dialogue" | "travel" | "rest";

export const PHASE_LABELS: Record<PhaseId, string> = {
  exploration: "Exploration",
  combat: "Combat",
  dialogue: "Dialogue",
  travel: "Voyage",
  rest: "Repos",
};

export const AUDIO_TYPE_LABELS: Record<"ambience" | "music" | "sound", string> = {
  ambience: "Ambiance",
  music: "Musique",
  sound: "Bruitage",
};

export const EFFECT_CATEGORY_LABELS: Record<"combat" | "narration" | "sensory", string> = {
  combat: "Combat",
  narration: "Narration",
  sensory: "Sensoriel",
};

/** Libellé d'un état : catalogue standard, sinon l'ID brut. */
export function conditionLabel(id: string): string {
  return (CONDITION_LABELS as Record<string, string>)[id] ?? id;
}

export const OUTCOME_LABELS: Record<"success" | "fail" | "partial" | "none", string> = {
  success: "réussite",
  fail: "échec",
  partial: "partiel",
  none: "info",
};
