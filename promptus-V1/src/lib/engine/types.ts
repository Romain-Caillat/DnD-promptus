// ============================================================================
// Promptus Game Engine — Core Types
// ----------------------------------------------------------------------------
// Pure TypeScript, framework-free. The schema mirror lives in
// src/lib/validation/entity-schemas.ts (Zod). Both must stay in sync.
// ============================================================================

// ----------------------------------------------------------------------------
// Primitive vocabulary
// ----------------------------------------------------------------------------

/** Caractéristiques du préréglage D&D 5e. Les effets acceptent tout id du ruleset. */
export type Stat = "STR" | "DEX" | "CON" | "INT" | "WIS" | "CHA";

/** Identifiant de caractéristique défini par le ruleset de la campagne. */
export type AbilityId = string;

export type DamageType =
  | "slashing"
  | "piercing"
  | "bludgeoning"
  | "fire"
  | "cold"
  | "lightning"
  | "thunder"
  | "acid"
  | "poison"
  | "psychic"
  | "necrotic"
  | "radiant"
  | "force";

export type Visibility =
  | "public"
  | "mj_only"
  | "players_in_session"
  | "specific_users";

export type EntityType =
  | "spell"
  | "item"
  | "npc"
  | "monster"
  | "character"
  | "location"
  | "event"
  | "condition"
  | "faction";

// ----------------------------------------------------------------------------
// Target specs
// ----------------------------------------------------------------------------

export type TargetSpec =
  | { type: "self" }
  | { type: "caster" }
  | { type: "single"; entityId: string }
  | { type: "multiple"; entityIds: string[] }
  | { type: "all_in_area" }
  | { type: "all_players" }
  | { type: "all_enemies" };

// ----------------------------------------------------------------------------
// Resources
// ----------------------------------------------------------------------------

export type ResourceKind =
  | "spell_slot"
  | "ki"
  | "rage"
  | "sorcery_point"
  | "bardic_inspiration"
  | "channel_divinity"
  | "action_surge"
  | "second_wind"
  | "custom";

// ----------------------------------------------------------------------------
// 20 primitive Effects (discriminated union by `type`)
// ----------------------------------------------------------------------------

export interface DamageEffect {
  type: "damage";
  amount: string; // dice notation: "1d8+3", "8d6/2"
  /** Id d'un type de dégâts du ruleset (préréglage : DamageType). */
  damageType: string;
  target: TargetSpec;
}

export interface HealEffect {
  type: "heal";
  amount: string;
  target: TargetSpec;
}

export interface ApplyConditionEffect {
  type: "apply_condition";
  conditionId: string;
  duration?: { rounds?: number; minutes?: number };
  target: TargetSpec;
  save?: { stat: AbilityId; dc: string | number };
}

export interface RemoveConditionEffect {
  type: "remove_condition";
  conditionId: string;
  target: TargetSpec;
}

export interface ModifyStatEffect {
  type: "modify_stat";
  stat: string; // "STR" | "AC" | "speed" | custom
  modifier: number;
  duration?: { rounds?: number; minutes?: number };
  target: TargetSpec;
}

export interface RollCheckEffect {
  type: "roll_check";
  stat: AbilityId;
  dc: string | number;
  target: TargetSpec;
  outcomeSuccess?: Effect[];
  outcomeFail?: Effect[];
}

export interface ConsumeResourceEffect {
  type: "consume_resource";
  resource: string;
  amount: number;
  level?: number; // for spell slots
  target: TargetSpec;
}

export interface RestoreResourceEffect {
  type: "restore_resource";
  resource: string;
  amount: number;
  level?: number;
  target: TargetSpec;
}

export interface SetStateEffect {
  type: "set_state";
  entityId: string;
  attribute: string;
  value: unknown;
}

export interface MoveEntityEffect {
  type: "move_entity";
  entityId: string;
  toLocationId: string;
}

export interface RevealEntityEffect {
  type: "reveal_entity";
  entityId: string;
  toUsers: "all_players" | "mj_only" | string[];
}

export interface SetRelationEffect {
  type: "set_relation";
  fromId: string;
  toId: string;
  disposition?: string;
  delta: number;
}

export interface TriggerEventEffect {
  type: "trigger_event";
  eventId: string;
}

export interface AddToInventoryEffect {
  type: "add_to_inventory";
  itemId: string;
  target: TargetSpec;
  quantity?: number;
}

export interface RemoveFromInventoryEffect {
  type: "remove_from_inventory";
  itemId: string;
  target: TargetSpec;
  quantity?: number;
}

export interface PlayAmbienceEffect {
  type: "play_ambience";
  ambienceId: string;
}

export interface PlayMusicEffect {
  type: "play_music";
  musicId: string;
}

export interface PlaySoundEffect {
  type: "play_sound";
  soundId: string;
}

export interface DisplayImageEffect {
  type: "display_image";
  imageId?: string;
  prompt?: string;
}

export interface DisplayTextEffect {
  type: "display_text";
  text: string;
}

// --- Effets narratifs sur l'état du monde (scénario V2) ---------------------

export interface SetFlagEffect {
  type: "set_flag";
  flag: string;
  value: unknown;
}

export interface AdvanceFrontEffect {
  type: "advance_front";
  frontId: string;
  /** Nombre d'étapes (défaut 1, négatif pour reculer). */
  steps?: number;
}

export interface RevealClueEffect {
  type: "reveal_clue";
  clueId: string;
}

export interface EnterSceneEffect {
  type: "enter_scene";
  sceneId: string;
}

export interface SetSceneStatusEffect {
  type: "set_scene_status";
  sceneId: string;
  status: "available" | "visited" | "resolved";
}

export type Effect =
  | DamageEffect
  | HealEffect
  | ApplyConditionEffect
  | RemoveConditionEffect
  | ModifyStatEffect
  | RollCheckEffect
  | ConsumeResourceEffect
  | RestoreResourceEffect
  | SetStateEffect
  | MoveEntityEffect
  | RevealEntityEffect
  | SetRelationEffect
  | TriggerEventEffect
  | AddToInventoryEffect
  | RemoveFromInventoryEffect
  | PlayAmbienceEffect
  | PlayMusicEffect
  | PlaySoundEffect
  | DisplayImageEffect
  | DisplayTextEffect
  | SetFlagEffect
  | AdvanceFrontEffect
  | RevealClueEffect
  | EnterSceneEffect
  | SetSceneStatusEffect;

export type EffectType = Effect["type"];

// ----------------------------------------------------------------------------
// Entity types
// ----------------------------------------------------------------------------

export interface BaseEntity {
  id: string;
  campaignId: string;
  type: EntityType;
  name: string;
  description?: string;
  imageUrl?: string;
  tags: string[];
  attributes: Record<string, unknown>;
  effects: Effect[];
  visibility: Visibility;
  version: number;
}

export interface SpellAttributes {
  level: number;
  school: string;
  castingTime: string;
  range: string;
  components: ("V" | "S" | "M")[];
  duration: string;
  save?: { stat: Stat; dcSource: string };
}

export interface CharacterAttributes {
  hp: number;
  hpMax: number;
  ac: number;
  speed?: number;
  level?: number;
  classes?: { name: string; level: number }[];
  abilityScores?: Record<Stat, number>;
  proficiencyBonus?: number;
  initiativeBonus?: number;
  inventory?: string[]; // entity IDs
  spellSlots?: Record<number, { current: number; max: number }>;
}

export interface MonsterAttributes extends CharacterAttributes {
  challengeRating?: number;
  size?: string;
  alignment?: string;
}

export interface NpcAttributes {
  hp?: number;
  hpMax?: number;
  ac?: number;
  faction?: string;
  currentLocation?: string;
  status?: "alive" | "wounded" | "dead" | "missing";
  motivation?: string;
  secret?: string;
  voiceActorRecommended?: string;
}

export interface ItemAttributes {
  rarity?: "common" | "uncommon" | "rare" | "very_rare" | "legendary";
  weight?: number;
  value?: number;
  attunement?: boolean;
  equipped?: boolean;
}

export interface LocationAttributes {
  parentLocation?: string;
  defaultAmbienceId?: string;
  description?: string;
}

export interface FactionAttributes {
  reputation?: number;
}

export interface ConditionDefinition {
  id: string;
  name: string;
  description: string;
  modifiers: ConditionModifier[];
}

export interface ConditionModifier {
  trigger:
    | "outgoing_attack"
    | "incoming_attack"
    | "outgoing_save"
    | "incoming_save"
    | "outgoing_check"
    | "incoming_check"
    | "outgoing_attack_within_5ft"
    | "incoming_attack_within_5ft"
    | "incapacitated";
  saveStat?: AbilityId[];
  effect:
    | "advantage"
    | "disadvantage"
    | "auto_fail"
    | "auto_success"
    | "auto_critical"
    | "true";
}

// ----------------------------------------------------------------------------
// Session-time entity state (mutable per session)
// ----------------------------------------------------------------------------

export interface ActiveCondition {
  conditionId: string;
  remainingRounds?: number;
  remainingMinutes?: number;
  source?: string; // entity id that applied
}

export interface EntityState {
  hp?: number;
  hpMax?: number;
  ac?: number;
  position?: { locationId?: string; relative?: string };
  conditions: ActiveCondition[];
  resources?: Record<string, { current: number; max: number }>;
  inventory?: string[];
  visible?: boolean;
}

// ----------------------------------------------------------------------------
// Initiative / turn order
// ----------------------------------------------------------------------------

export interface InitiativeEntry {
  entityId: string;
  initiative: number;
  isPlayer: boolean;
}

// ----------------------------------------------------------------------------
// Resolution records — what every effect emits for the timeline
// ----------------------------------------------------------------------------

export interface RollDetail {
  notation: string;
  result: number;
  rolls?: number[];
  modifier?: number;
  advantage?: boolean;
  disadvantage?: boolean;
}

export interface AppliedMutation {
  entityId: string;
  field: string;
  before: unknown;
  after: unknown;
}

export interface ResolutionRecord {
  effect: Effect;
  rolls: RollDetail[];
  outcome: "success" | "fail" | "partial" | "none";
  applied: AppliedMutation[];
  description: string;
  timestamp: string;
}

// ----------------------------------------------------------------------------
// Style guide for image generation
// ----------------------------------------------------------------------------

export interface StyleGuide {
  artStyle?: string;
  palette?: string;
  mood?: string;
  promptPrefix?: string;
  promptSuffix?: string;
  negativePrompt?: string;
}
