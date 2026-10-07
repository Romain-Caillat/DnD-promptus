-- Phase 5 · between two sessions: the recaps the co-GM drafts and the GM
-- publishes (session/write-recaps), and the launch of the next session,
-- « Précédemment… » read sentence by sentence (gm/launch-session).

-- The world when the session went live: what the recap's facts compare
-- the end against. NULL for a session ended from the lobby, or played
-- before this migration: the previous session's `world_at_end` stands in.
ALTER TABLE game_sessions ADD COLUMN world_at_start JSONB;

-- The chronicle entry of the session: a title and one or two lines.
-- Player text once published, like « Précédemment… ».
ALTER TABLE game_sessions ADD COLUMN chronicle_title TEXT NOT NULL DEFAULT '';
ALTER TABLE game_sessions ADD COLUMN chronicle TEXT NOT NULL DEFAULT '';

-- « Précédemment… » and the chronicle entry reach the players only once
-- the GM published them; until then they are drafts. Sessions ended
-- before this migration published « Précédemment… » at the end.
ALTER TABLE game_sessions ADD COLUMN published_at TIMESTAMPTZ;
UPDATE game_sessions SET published_at = ended_at
  WHERE status = 'ended' AND previously <> '';

-- gm/launch-session: how many sentences of the last published
-- « Précédemment… » the GM has shown the table. NULL when no reading is
-- under way: before the start, and once the first scene is shown.
ALTER TABLE game_sessions ADD COLUMN previously_shown INTEGER
  CHECK (previously_shown >= 0);
