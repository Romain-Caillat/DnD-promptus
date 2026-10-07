-- engine/save-against-death, player/face-death — a character whose death
-- the GM confirmed stays in the campaign, marked dead with the evening
-- it fell in and its last words; its player may then create another
-- character, who starts at the group's level (`start_xp`).

ALTER TABLE characters DROP CONSTRAINT characters_status_check;
ALTER TABLE characters ADD CONSTRAINT characters_status_check
  CHECK (status IN ('draft', 'submitted', 'validated', 'returned', 'dead'));

-- One living character per player; the dead stay behind.
DROP INDEX characters_player_idx;
CREATE UNIQUE INDEX characters_player_idx ON characters (player_id) WHERE status <> 'dead';

ALTER TABLE characters
  ADD COLUMN died_at      TIMESTAMPTZ,
  ADD COLUMN died_session UUID REFERENCES game_sessions(id) ON DELETE SET NULL,
  ADD COLUMN last_words   TEXT NOT NULL DEFAULT '' CHECK (char_length(last_words) <= 280),
  -- XP a replacement character enters play with: the group's level.
  ADD COLUMN start_xp     INT NOT NULL DEFAULT 0 CHECK (start_xp >= 0),
  ADD CONSTRAINT characters_dead_check CHECK ((status = 'dead') = (died_at IS NOT NULL));
