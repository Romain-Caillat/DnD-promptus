-- session/write-recaps — the recap is read by the GM before it leaves.
--
-- Ending a session measures what it changed (`facts`, from the world at
-- its start to the world at its end: `promptus_shared::story::recap`)
-- and drafts both recaps from it when the GM wrote none. Players read
-- « Précédemment… » and the chronicle entry (`title`, `chronicle`) only
-- once the GM publishes (`recap_status`).
ALTER TABLE game_sessions
  -- The world when the lobby opened: what `facts` is measured from.
  ADD COLUMN world_at_start JSONB,
  -- `RecapFacts`, GM-side (it names the fronts that moved).
  ADD COLUMN facts          JSONB,
  -- The chronicle's entry: a title and two lines. Player text once
  -- published.
  ADD COLUMN title          TEXT NOT NULL DEFAULT '',
  ADD COLUMN chronicle      TEXT NOT NULL DEFAULT '',
  ADD COLUMN recap_status   TEXT NOT NULL DEFAULT 'draft'
                            CHECK (recap_status IN ('draft', 'published')),
  ADD COLUMN published_at   TIMESTAMPTZ;

-- Sessions ended before this step: their « Précédemment… » was already
-- shown to the players, so it stays shown.
UPDATE game_sessions SET recap_status = 'published', published_at = ended_at
WHERE status = 'ended' AND previously <> '';

-- session/schedule-sessions — the next session's date. The GM proposes
-- a few start times, the players say which suit them, the GM picks
-- one; the lobby then opens by itself `lobby_minutes` before it. The
-- plan is consumed when that lobby opens.
CREATE TABLE session_plans (
  campaign_id   UUID PRIMARY KEY REFERENCES campaigns(id) ON DELETE CASCADE,
  options       TIMESTAMPTZ[] NOT NULL DEFAULT '{}',
  chosen_at     TIMESTAMPTZ,
  lobby_minutes INTEGER NOT NULL DEFAULT 15 CHECK (lobby_minutes BETWEEN 0 AND 180),
  created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TRIGGER session_plans_set_updated_at
  BEFORE UPDATE ON session_plans
  FOR EACH ROW EXECUTE FUNCTION set_updated_at();

-- Which proposed times suit each player (a subset of `options`).
CREATE TABLE session_availability (
  campaign_id UUID NOT NULL REFERENCES session_plans(campaign_id) ON DELETE CASCADE,
  player_id   UUID NOT NULL REFERENCES players(id) ON DELETE CASCADE,
  available   TIMESTAMPTZ[] NOT NULL DEFAULT '{}',
  updated_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
  PRIMARY KEY (campaign_id, player_id)
);
