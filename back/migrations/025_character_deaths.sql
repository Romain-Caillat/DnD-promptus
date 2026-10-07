-- engine/save-against-death, player/face-death — a character's death,
-- always confirmed by the GM (the dice of a `death_saves` rule propose
-- it, or the GM decides it), then what its player says and does next.
--
-- The dead character stays a row of `characters`, status `fallen`: its
-- sheet, history and hooks are kept for the chronicle. Its player may
-- make a new character, so « one character per player » now holds for
-- the living only. NOTE for whoever merges a lot that also widens the
-- status list: keep every status of both.
ALTER TABLE characters DROP CONSTRAINT characters_status_check;
ALTER TABLE characters ADD CONSTRAINT characters_status_check
  CHECK (status IN ('draft', 'submitted', 'validated', 'returned', 'fallen'));

DROP INDEX characters_player_idx;
CREATE UNIQUE INDEX characters_player_idx ON characters (player_id) WHERE status <> 'fallen';

-- A replacement character joins at the party's level: the XP it starts
-- with, put into its play state when the GM validates it.
ALTER TABLE characters
  ADD COLUMN starting_xp INT NOT NULL DEFAULT 0 CHECK (starting_xp >= 0);

-- One row per dead character. `last_words` are the player's, shown to
-- the whole table (and kept for the chronicle, session/write-recaps);
-- `next` is their choice afterwards: watch the evening, make a new
-- character, or wait for a hook from the GM.
CREATE TABLE character_deaths (
  character_id UUID PRIMARY KEY REFERENCES characters(id) ON DELETE CASCADE,
  campaign_id  UUID NOT NULL REFERENCES campaigns(id) ON DELETE CASCADE,
  player_id    UUID NOT NULL REFERENCES players(id) ON DELETE CASCADE,
  session_id   UUID REFERENCES game_sessions(id) ON DELETE SET NULL,
  -- `rules`: the death saves proposed it; `gm`: the GM decided it.
  cause        TEXT NOT NULL CHECK (cause IN ('rules', 'gm')),
  level        INT NOT NULL,
  -- The scene it happened in, when there was one.
  node         TEXT,
  last_words   TEXT,
  said_at      TIMESTAMPTZ,
  next         TEXT CHECK (next IN ('watch', 'new', 'hook')),
  died_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX character_deaths_campaign_idx ON character_deaths (campaign_id, died_at);
CREATE INDEX character_deaths_player_idx ON character_deaths (player_id, died_at DESC);
