DROP TABLE "scene_markers" CASCADE;--> statement-breakpoint
ALTER TABLE "campaigns" ADD COLUMN "story" jsonb;--> statement-breakpoint
DROP TYPE "public"."marker_status";