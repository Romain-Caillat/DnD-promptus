import {
  pgTable,
  text,
  jsonb,
  timestamp,
  integer,
  boolean,
  pgEnum,
  index,
  uniqueIndex,
} from "drizzle-orm/pg-core";
import type {
  Effect,
  StyleGuide,
  EntityState,
  ResolutionRecord,
  InitiativeEntry,
} from "@/lib/engine/types";
import type { Ruleset } from "@/lib/engine/ruleset";
import type { CampaignStory } from "@/lib/engine/story";
import type { WorldState } from "@/lib/engine/world";
import type { LlmUsage } from "@/lib/ai/llm";
import type {
  GenerationInput,
  GenerationResult,
  GenerationStep,
  JobStatus,
} from "@/lib/generation/types";

export interface AiSettings {
  model?: string;
  budgetUsd?: number;
}

// ============================================================================
// Enums
// ============================================================================

export const entityTypeEnum = pgEnum("entity_type", [
  "spell",
  "item",
  "npc",
  "monster",
  "character",
  "location",
  "event",
  "condition",
  "faction",
]);

export const visibilityEnum = pgEnum("visibility", [
  "public",
  "mj_only",
  "players_in_session",
  "specific_users",
]);

export const phaseEnum = pgEnum("phase", [
  "exploration",
  "combat",
  "dialogue",
  "travel",
  "rest",
]);

export const audioTypeEnum = pgEnum("audio_type", [
  "ambience",
  "music",
  "sound",
]);


// ============================================================================
// Tables
// ============================================================================

export const campaigns = pgTable("campaigns", {
  id: text("id").primaryKey(),
  name: text("name").notNull(),
  description: text("description"),
  styleGuide: jsonb("style_guide").$type<StyleGuide>().default({}).notNull(),
  systemTemplate: text("system_template").default("dnd5e").notNull(),
  /** Système de règles défini par le MJ ; null = préréglage D&D 5e. */
  ruleset: jsonb("ruleset").$type<Ruleset>(),
  /** Histoire structurée (bible, fronts, scènes, indices, cartes) ; null = vide. */
  story: jsonb("story").$type<CampaignStory>(),
  /** État du monde (scène courante, indices, menaces…), partagé entre sessions. */
  worldState: jsonb("world_state").$type<WorldState>(),
  /** Réglages IA : modèle OpenRouter et budget total de génération (dollars). */
  aiSettings: jsonb("ai_settings").$type<AiSettings>().default({}).notNull(),
  createdAt: timestamp("created_at", { withTimezone: true }).defaultNow().notNull(),
  updatedAt: timestamp("updated_at", { withTimezone: true }).defaultNow().notNull(),
});

export const entities = pgTable(
  "entities",
  {
    id: text("id").primaryKey(),
    campaignId: text("campaign_id")
      .references(() => campaigns.id, { onDelete: "cascade" })
      .notNull(),
    type: entityTypeEnum("type").notNull(),
    name: text("name").notNull(),
    description: text("description"),
    imageUrl: text("image_url"),
    tags: jsonb("tags").$type<string[]>().default([]).notNull(),
    attributes: jsonb("attributes")
      .$type<Record<string, unknown>>()
      .default({})
      .notNull(),
    effects: jsonb("effects").$type<Effect[]>().default([]).notNull(),
    visibility: visibilityEnum("visibility").default("public").notNull(),
    version: integer("version").default(1).notNull(),
    createdAt: timestamp("created_at", { withTimezone: true })
      .defaultNow()
      .notNull(),
    updatedAt: timestamp("updated_at", { withTimezone: true })
      .defaultNow()
      .notNull(),
  },
  (t) => [
    index("entities_campaign_type_idx").on(t.campaignId, t.type),
    index("entities_name_idx").on(t.name),
  ],
);

export const sessions = pgTable("sessions", {
  id: text("id").primaryKey(),
  campaignId: text("campaign_id")
    .references(() => campaigns.id, { onDelete: "cascade" })
    .notNull(),
  name: text("name").notNull(),
  currentPhase: phaseEnum("current_phase").default("exploration").notNull(),
  initiativeOrder: jsonb("initiative_order")
    .$type<InitiativeEntry[]>()
    .default([])
    .notNull(),
  combatRound: integer("combat_round").default(0).notNull(),
  activeTurnIndex: integer("active_turn_index").default(0).notNull(),
  /** Code du lien d'invitation des joueurs (/play/<code>). */
  inviteCode: text("invite_code").unique(),
  startedAt: timestamp("started_at", { withTimezone: true })
    .defaultNow()
    .notNull(),
  endedAt: timestamp("ended_at", { withTimezone: true }),
});

export const sessionState = pgTable(
  "session_state",
  {
    id: text("id").primaryKey(),
    sessionId: text("session_id")
      .references(() => sessions.id, { onDelete: "cascade" })
      .notNull(),
    entityId: text("entity_id")
      .references(() => entities.id, { onDelete: "cascade" })
      .notNull(),
    currentState: jsonb("current_state").$type<EntityState>().notNull(),
    updatedAt: timestamp("updated_at", { withTimezone: true })
      .defaultNow()
      .notNull(),
  },
  (t) => [uniqueIndex("session_state_unique").on(t.sessionId, t.entityId)],
);

export const sessionTimeline = pgTable("session_timeline", {
  id: text("id").primaryKey(),
  sessionId: text("session_id")
    .references(() => sessions.id, { onDelete: "cascade" })
    .notNull(),
  round: integer("round"),
  description: text("description").notNull(),
  resolutionRecord: jsonb("resolution_record").$type<ResolutionRecord>(),
  /** Visible des joueurs (sinon réservé au MJ). */
  isPublic: boolean("is_public").default(true).notNull(),
  createdAt: timestamp("created_at", { withTimezone: true })
    .defaultNow()
    .notNull(),
});


/** Joueurs ayant rejoint une session à distance. */
export const sessionPlayers = pgTable(
  "session_players",
  {
    id: text("id").primaryKey(),
    sessionId: text("session_id")
      .references(() => sessions.id, { onDelete: "cascade" })
      .notNull(),
    name: text("name").notNull(),
    /** Personnage incarné ; null = spectateur. */
    characterEntityId: text("character_entity_id").references(() => entities.id, { onDelete: "set null" }),
    /** Empreinte SHA-256 du jeton secret remis au joueur. */
    tokenHash: text("token_hash").notNull(),
    createdAt: timestamp("created_at", { withTimezone: true }).defaultNow().notNull(),
    lastSeenAt: timestamp("last_seen_at", { withTimezone: true }).defaultNow().notNull(),
  },
  (t) => [index("session_players_session_idx").on(t.sessionId), uniqueIndex("session_players_token_idx").on(t.tokenHash)],
);

/** Actions demandées par les joueurs, validées par le MJ. */
export const playerRequests = pgTable(
  "player_requests",
  {
    id: text("id").primaryKey(),
    sessionId: text("session_id")
      .references(() => sessions.id, { onDelete: "cascade" })
      .notNull(),
    playerId: text("player_id")
      .references(() => sessionPlayers.id, { onDelete: "cascade" })
      .notNull(),
    actionId: text("action_id").notNull(),
    label: text("label").notNull(),
    targetIds: jsonb("target_ids").$type<string[]>().default([]).notNull(),
    note: text("note"),
    status: text("status").$type<"pending" | "resolved" | "rejected">().default("pending").notNull(),
    result: text("result"),
    createdAt: timestamp("created_at", { withTimezone: true }).defaultNow().notNull(),
    resolvedAt: timestamp("resolved_at", { withTimezone: true }),
  },
  (t) => [index("player_requests_session_idx").on(t.sessionId, t.status)],
);

export const generationJobs = pgTable(
  "generation_jobs",
  {
    id: text("id").primaryKey(),
    campaignId: text("campaign_id")
      .references(() => campaigns.id, { onDelete: "cascade" })
      .notNull(),
    status: text("status").$type<JobStatus>().notNull(),
    input: jsonb("input").$type<GenerationInput>().notNull(),
    steps: jsonb("steps").$type<GenerationStep[]>().notNull(),
    result: jsonb("result").$type<GenerationResult>(),
    error: text("error"),
    usage: jsonb("usage").$type<LlmUsage>().notNull(),
    model: text("model").notNull(),
    createdAt: timestamp("created_at", { withTimezone: true }).defaultNow().notNull(),
    updatedAt: timestamp("updated_at", { withTimezone: true }).defaultNow().notNull(),
  },
  (t) => [index("generation_jobs_campaign_idx").on(t.campaignId, t.createdAt)],
);

export const audioAssets = pgTable("audio_assets", {
  id: text("id").primaryKey(),
  name: text("name").notNull(),
  type: audioTypeEnum("type").notNull(),
  filePath: text("file_path").notNull(),
  durationSeconds: integer("duration_seconds"),
  tags: jsonb("tags").$type<string[]>().default([]).notNull(),
  license: text("license"),
  attribution: text("attribution"),
});

// ============================================================================
// Inferred types
// ============================================================================

export type Campaign = typeof campaigns.$inferSelect;
export type NewCampaign = typeof campaigns.$inferInsert;
export type EntityRow = typeof entities.$inferSelect;
export type NewEntityRow = typeof entities.$inferInsert;
export type Session = typeof sessions.$inferSelect;
export type NewSession = typeof sessions.$inferInsert;
export type SessionStateRow = typeof sessionState.$inferSelect;
export type SessionTimelineRow = typeof sessionTimeline.$inferSelect;
export type AudioAsset = typeof audioAssets.$inferSelect;
export type GenerationJobRow = typeof generationJobs.$inferSelect;
export type SessionPlayerRow = typeof sessionPlayers.$inferSelect;
export type PlayerRequestRow = typeof playerRequests.$inferSelect;
