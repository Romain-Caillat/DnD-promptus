-- campaign/list-campaigns — what a campaign is besides its story.
--
-- The title, universe, pitch and player hook live in the story document
-- (`campaigns.story`): they travel with the YAML export and the player
-- projection reads them there. What stays here is the GM's own planning,
-- which no player ever sees and no export carries:
-- - `player_count`: how many players the campaign is prepared for (the
--   table is six at Romain's, `MEMORY.md` §6);
-- - `ai_budget_cents`: the most the campaign may spend on AI calls, in
--   US cents (OpenRouter bills in dollars). 0 means no AI spending at
--   all: a missing budget fails closed (`MEMORY.md` §3, every AI call is
--   counted);
-- - `archived_at`: set when the GM shelves the campaign. It leaves the
--   main list and nothing else changes: players keep their seats, and
--   the GM can take it off the shelf.

ALTER TABLE campaigns
  ADD COLUMN player_count    SMALLINT NOT NULL DEFAULT 6
    CHECK (player_count BETWEEN 1 AND 12),
  ADD COLUMN ai_budget_cents INTEGER NOT NULL DEFAULT 0
    CHECK (ai_budget_cents BETWEEN 0 AND 1000000),
  ADD COLUMN archived_at     TIMESTAMPTZ;
