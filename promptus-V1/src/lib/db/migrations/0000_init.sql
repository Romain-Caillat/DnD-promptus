CREATE TYPE "public"."audio_type" AS ENUM('ambience', 'music', 'sound');--> statement-breakpoint
CREATE TYPE "public"."entity_type" AS ENUM('spell', 'item', 'npc', 'monster', 'character', 'location', 'event', 'condition', 'faction');--> statement-breakpoint
CREATE TYPE "public"."marker_status" AS ENUM('armed', 'triggered', 'disabled');--> statement-breakpoint
CREATE TYPE "public"."phase" AS ENUM('exploration', 'combat', 'dialogue', 'travel', 'rest');--> statement-breakpoint
CREATE TYPE "public"."visibility" AS ENUM('public', 'mj_only', 'players_in_session', 'specific_users');--> statement-breakpoint
CREATE TABLE "audio_assets" (
	"id" text PRIMARY KEY NOT NULL,
	"name" text NOT NULL,
	"type" "audio_type" NOT NULL,
	"file_path" text NOT NULL,
	"duration_seconds" integer,
	"tags" jsonb DEFAULT '[]'::jsonb NOT NULL,
	"license" text,
	"attribution" text
);
--> statement-breakpoint
CREATE TABLE "campaigns" (
	"id" text PRIMARY KEY NOT NULL,
	"name" text NOT NULL,
	"description" text,
	"style_guide" jsonb DEFAULT '{}'::jsonb NOT NULL,
	"system_template" text DEFAULT 'dnd5e' NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	"updated_at" timestamp with time zone DEFAULT now() NOT NULL
);
--> statement-breakpoint
CREATE TABLE "entities" (
	"id" text PRIMARY KEY NOT NULL,
	"campaign_id" text NOT NULL,
	"type" "entity_type" NOT NULL,
	"name" text NOT NULL,
	"description" text,
	"image_url" text,
	"tags" jsonb DEFAULT '[]'::jsonb NOT NULL,
	"attributes" jsonb DEFAULT '{}'::jsonb NOT NULL,
	"effects" jsonb DEFAULT '[]'::jsonb NOT NULL,
	"visibility" "visibility" DEFAULT 'public' NOT NULL,
	"version" integer DEFAULT 1 NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	"updated_at" timestamp with time zone DEFAULT now() NOT NULL
);
--> statement-breakpoint
CREATE TABLE "scene_markers" (
	"id" text PRIMARY KEY NOT NULL,
	"campaign_id" text NOT NULL,
	"name" text NOT NULL,
	"description" text,
	"conditions" jsonb NOT NULL,
	"effects" jsonb NOT NULL,
	"status" "marker_status" DEFAULT 'armed' NOT NULL,
	"one_shot" boolean DEFAULT true NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	"triggered_at" timestamp with time zone
);
--> statement-breakpoint
CREATE TABLE "session_state" (
	"id" text PRIMARY KEY NOT NULL,
	"session_id" text NOT NULL,
	"entity_id" text NOT NULL,
	"current_state" jsonb NOT NULL,
	"updated_at" timestamp with time zone DEFAULT now() NOT NULL
);
--> statement-breakpoint
CREATE TABLE "session_timeline" (
	"id" text PRIMARY KEY NOT NULL,
	"session_id" text NOT NULL,
	"round" integer,
	"description" text NOT NULL,
	"resolution_record" jsonb,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL
);
--> statement-breakpoint
CREATE TABLE "sessions" (
	"id" text PRIMARY KEY NOT NULL,
	"campaign_id" text NOT NULL,
	"name" text NOT NULL,
	"current_phase" "phase" DEFAULT 'exploration' NOT NULL,
	"initiative_order" jsonb DEFAULT '[]'::jsonb NOT NULL,
	"combat_round" integer DEFAULT 0 NOT NULL,
	"active_turn_index" integer DEFAULT 0 NOT NULL,
	"started_at" timestamp with time zone DEFAULT now() NOT NULL,
	"ended_at" timestamp with time zone
);
--> statement-breakpoint
ALTER TABLE "entities" ADD CONSTRAINT "entities_campaign_id_campaigns_id_fk" FOREIGN KEY ("campaign_id") REFERENCES "public"."campaigns"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "scene_markers" ADD CONSTRAINT "scene_markers_campaign_id_campaigns_id_fk" FOREIGN KEY ("campaign_id") REFERENCES "public"."campaigns"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "session_state" ADD CONSTRAINT "session_state_session_id_sessions_id_fk" FOREIGN KEY ("session_id") REFERENCES "public"."sessions"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "session_state" ADD CONSTRAINT "session_state_entity_id_entities_id_fk" FOREIGN KEY ("entity_id") REFERENCES "public"."entities"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "session_timeline" ADD CONSTRAINT "session_timeline_session_id_sessions_id_fk" FOREIGN KEY ("session_id") REFERENCES "public"."sessions"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "sessions" ADD CONSTRAINT "sessions_campaign_id_campaigns_id_fk" FOREIGN KEY ("campaign_id") REFERENCES "public"."campaigns"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
CREATE INDEX "entities_campaign_type_idx" ON "entities" USING btree ("campaign_id","type");--> statement-breakpoint
CREATE INDEX "entities_name_idx" ON "entities" USING btree ("name");--> statement-breakpoint
CREATE UNIQUE INDEX "session_state_unique" ON "session_state" USING btree ("session_id","entity_id");