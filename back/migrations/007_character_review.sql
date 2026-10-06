-- session/validate-characters — the GM reviews each sheet, validates it
-- or returns it with a word, then reviews only what changed; secret
-- hooks drawn from the players' backstories.

-- The sheet as the GM last saw it when they validated or returned it.
-- When the player sends the sheet again, the GM's review shows the
-- difference between this snapshot and `sheet`, nothing else. GM-side:
-- it never goes into the player projection.
ALTER TABLE characters
  ADD COLUMN reviewed_sheet JSONB,
  ADD COLUMN reviewed_at    TIMESTAMPTZ;

-- What the GM draws from a player's backstory to weave into the
-- campaign (« Dorn, le frère de Borin, est le mort qui marche »).
-- GM-only, always: no player route reads this table, and
-- `tests/player_routes_test.rs` proves none of it leaks. `links` holds
-- the ids of the story's nodes and fronts the hook ties into.
CREATE TABLE secret_hooks (
  id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  campaign_id  UUID NOT NULL REFERENCES campaigns(id) ON DELETE CASCADE,
  character_id UUID NOT NULL REFERENCES characters(id) ON DELETE CASCADE,
  title        TEXT NOT NULL,
  body         TEXT NOT NULL DEFAULT '',
  links        JSONB NOT NULL DEFAULT '[]'::jsonb,
  created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at   TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX secret_hooks_campaign_idx ON secret_hooks (campaign_id, created_at);

CREATE TRIGGER secret_hooks_set_updated_at
  BEFORE UPDATE ON secret_hooks
  FOR EACH ROW EXECUTE FUNCTION set_updated_at();
