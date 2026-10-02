CREATE TABLE "player_requests" (
	"id" text PRIMARY KEY NOT NULL,
	"session_id" text NOT NULL,
	"player_id" text NOT NULL,
	"action_id" text NOT NULL,
	"label" text NOT NULL,
	"target_ids" jsonb DEFAULT '[]'::jsonb NOT NULL,
	"note" text,
	"status" text DEFAULT 'pending' NOT NULL,
	"result" text,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	"resolved_at" timestamp with time zone
);
--> statement-breakpoint
CREATE TABLE "session_players" (
	"id" text PRIMARY KEY NOT NULL,
	"session_id" text NOT NULL,
	"name" text NOT NULL,
	"character_entity_id" text,
	"token_hash" text NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	"last_seen_at" timestamp with time zone DEFAULT now() NOT NULL
);
--> statement-breakpoint
ALTER TABLE "session_timeline" ADD COLUMN "is_public" boolean DEFAULT true NOT NULL;--> statement-breakpoint
ALTER TABLE "sessions" ADD COLUMN "invite_code" text;--> statement-breakpoint
ALTER TABLE "player_requests" ADD CONSTRAINT "player_requests_session_id_sessions_id_fk" FOREIGN KEY ("session_id") REFERENCES "public"."sessions"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "player_requests" ADD CONSTRAINT "player_requests_player_id_session_players_id_fk" FOREIGN KEY ("player_id") REFERENCES "public"."session_players"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "session_players" ADD CONSTRAINT "session_players_session_id_sessions_id_fk" FOREIGN KEY ("session_id") REFERENCES "public"."sessions"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "session_players" ADD CONSTRAINT "session_players_character_entity_id_entities_id_fk" FOREIGN KEY ("character_entity_id") REFERENCES "public"."entities"("id") ON DELETE set null ON UPDATE no action;--> statement-breakpoint
CREATE INDEX "player_requests_session_idx" ON "player_requests" USING btree ("session_id","status");--> statement-breakpoint
CREATE INDEX "session_players_session_idx" ON "session_players" USING btree ("session_id");--> statement-breakpoint
CREATE UNIQUE INDEX "session_players_token_idx" ON "session_players" USING btree ("token_hash");--> statement-breakpoint
ALTER TABLE "sessions" ADD CONSTRAINT "sessions_invite_code_unique" UNIQUE("invite_code");