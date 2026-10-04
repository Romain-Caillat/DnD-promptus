-- campaign/model-story-graph — a GM's campaigns.
--
-- The prepared story is one JSONB document per campaign, typed by
-- `promptus_shared::story::Campaign` (validated by deserialisation on
-- every write and read); the living world is a second document, typed
-- by `WorldState`. Both are always read and written whole: the story is
-- edited as a document (YAML import/export, co-GM diffs) and the
-- validator and the player projection need all of it at once, so
-- splitting it into tables would buy joins and no query we need.
--
-- Concurrency: every world (or story) write locks this row with
-- `SELECT … FOR UPDATE` and re-reads it inside the transaction
-- (`MEMORY.md` §3, `back/src/campaigns.rs`).

CREATE TABLE campaigns (
  id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  gm_id      UUID NOT NULL REFERENCES gms(id) ON DELETE CASCADE,
  story      JSONB NOT NULL,
  world      JSONB NOT NULL DEFAULT '{}'::jsonb,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX campaigns_gm_idx ON campaigns (gm_id, updated_at DESC);

CREATE TRIGGER campaigns_set_updated_at
  BEFORE UPDATE ON campaigns
  FOR EACH ROW EXECUTE FUNCTION set_updated_at();
