-- platform/stream-prep-edits — one revision number per campaign and the
-- journal of its preparation writes.
--
-- Every write of the story (the GM's edits, an import, the co-GM's
-- workshop and generation, the settings, a rule version adopted) takes
-- the next `prep_revision` under the campaign lock and leaves one line
-- here: who wrote (`author`), which entities moved (`changes`, ids and
-- names, `story::revision::Touch`) and each of them as it was before
-- (`inverse`, `story::revision::Restore`), which is what an undo puts
-- back. `inverse` is NULL for what is not undone (a rule version the
-- server adopted when a session opened).
--
-- `target` says what the line wrote: `story` today; map operations
-- (`maps/edit-maps-by-ops`) join the same journal and the same number,
-- so a campaign has one revision system, never one per map.
--
-- The revision is mirrored in the GM-only `prep` live topic
-- (`back/src/live`): the GM's screens hear the number, never the data,
-- and fetch "what changed since N" over HTTP.

ALTER TABLE campaigns ADD COLUMN prep_revision BIGINT NOT NULL DEFAULT 0;

CREATE TABLE prep_revisions (
  campaign_id UUID NOT NULL REFERENCES campaigns(id) ON DELETE CASCADE,
  revision    BIGINT NOT NULL,
  author      TEXT NOT NULL CHECK (author IN ('gm', 'token', 'copilot', 'server')),
  target      TEXT NOT NULL DEFAULT 'story',
  changes     JSONB NOT NULL,
  inverse     JSONB,
  -- The revision this one took back, when it is an undo.
  undoes      BIGINT,
  created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
  PRIMARY KEY (campaign_id, revision)
);
