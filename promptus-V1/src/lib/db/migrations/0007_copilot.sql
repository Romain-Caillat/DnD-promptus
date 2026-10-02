CREATE TABLE "ai_calls" (
	"id" text PRIMARY KEY NOT NULL,
	"campaign_id" text NOT NULL,
	"session_id" text,
	"kind" text NOT NULL,
	"status" text NOT NULL,
	"model" text NOT NULL,
	"input" jsonb DEFAULT '{}'::jsonb NOT NULL,
	"output" jsonb,
	"error" text,
	"cost_usd" double precision DEFAULT 0 NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	"updated_at" timestamp with time zone DEFAULT now() NOT NULL
);
--> statement-breakpoint
ALTER TABLE "session_timeline" ADD COLUMN "kind" text DEFAULT 'event' NOT NULL;--> statement-breakpoint
ALTER TABLE "ai_calls" ADD CONSTRAINT "ai_calls_campaign_id_campaigns_id_fk" FOREIGN KEY ("campaign_id") REFERENCES "public"."campaigns"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "ai_calls" ADD CONSTRAINT "ai_calls_session_id_sessions_id_fk" FOREIGN KEY ("session_id") REFERENCES "public"."sessions"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
CREATE INDEX "ai_calls_campaign_idx" ON "ai_calls" USING btree ("campaign_id","created_at");--> statement-breakpoint
CREATE INDEX "ai_calls_session_idx" ON "ai_calls" USING btree ("session_id","created_at");