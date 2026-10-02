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
  MarkerCondition,
} from "@/lib/engine/types";
import type { Ruleset } from "@/lib/engine/ruleset";

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

export const markerStatusEnum = pgEnum("marker_status", [
  "armed",
  "triggered",
  "disabled",
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
  createdAt: timestamp("created_at", { withTimezone: true })
    .defaultNow()
    .notNull(),
});

export const sceneMarkers = pgTable("scene_markers", {
  id: text("id").primaryKey(),
  campaignId: text("campaign_id")
    .references(() => campaigns.id, { onDelete: "cascade" })
    .notNull(),
  name: text("name").notNull(),
  description: text("description"),
  conditions: jsonb("conditions").$type<MarkerCondition>().notNull(),
  effects: jsonb("effects").$type<Effect[]>().notNull(),
  status: markerStatusEnum("status").default("armed").notNull(),
  oneShot: boolean("one_shot").default(true).notNull(),
  createdAt: timestamp("created_at", { withTimezone: true })
    .defaultNow()
    .notNull(),
  triggeredAt: timestamp("triggered_at", { withTimezone: true }),
});

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
export type SceneMarker = typeof sceneMarkers.$inferSelect;
export type AudioAsset = typeof audioAssets.$inferSelect;
