-- platform/propose-edits-by-claude — Claude proposes, the GM keeps.
--
-- A write with the GM's personal token (Claude, through the MCP server)
-- no longer lands in the campaign: it becomes a batch of proposals, the
-- same `story_proposals` rows as the co-GM's workshop (one model of
-- batch, never two circuits):
-- - `author`: `copilot` (the workshop) or `token` (Claude);
-- - `prompt` holds the batch's title when Claude wrote it;
-- - `focus`: where on a map the batch is about (`story::Anchor`), if
--   Claude said so;
-- - `map_ops`: the batches of map operations it carries, each with the
--   revision Claude read the map at (`prep::claude::MapItem`);
-- - `decisions`: the GM's word on each item, by key (`e<i>` for the
--   i-th story edit, `m<i>` for the i-th map batch): kept, discarded
--   (with the GM's word, read back by Claude) or refused (it no longer
--   applied when the GM kept everything);
-- - `feedback`: the GM's word on the whole batch.
-- `status` stays `pending` while an item waits; then `accepted` when at
-- least one was kept, `rejected` otherwise.
ALTER TABLE story_proposals
  ADD COLUMN author TEXT NOT NULL DEFAULT 'copilot' CHECK (author IN ('copilot', 'token')),
  ADD COLUMN focus JSONB,
  ADD COLUMN map_ops JSONB NOT NULL DEFAULT '[]',
  ADD COLUMN decisions JSONB NOT NULL DEFAULT '{}',
  ADD COLUMN feedback TEXT NOT NULL DEFAULT '';

CREATE INDEX story_proposals_author_idx ON story_proposals (campaign_id, author, created_at DESC);

-- « Claude a la main »: the GM lets Claude's writes land directly (each
-- one a line of the preparation journal, undone like any other). Set by
-- the GM's session only, never while an evening is open; a write that
-- comes while one is open, or that touches what players already know,
-- is still proposed.
ALTER TABLE campaigns ADD COLUMN claude_hand BOOLEAN NOT NULL DEFAULT false;
