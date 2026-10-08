-- engine/level-up — what a level asked the player, kept because it was
-- a choice and a roll: the hit points of each level above the first (the
-- die or the average, `rules::level_up::HitPointGain`), and the last
-- level the player went through, above which the level-up screen waits
-- for them. Everything else a level brings (cards, upgrade points) is
-- derived from total XP, never stored (`players::play`).
ALTER TABLE character_play
  ADD COLUMN hit_point_gains JSONB NOT NULL DEFAULT '[]'::jsonb,
  ADD COLUMN level_seen      INT   NOT NULL DEFAULT 1 CHECK (level_seen >= 1);

-- The history gains a level's hit points taken (`level`) and a death the
-- GM confirmed (`death`, player/face-death). NOTE for whoever merges a
-- lot that also widens this list: keep every kind of both.
ALTER TABLE play_adjustments DROP CONSTRAINT play_adjustments_kind_check;
ALTER TABLE play_adjustments ADD CONSTRAINT play_adjustments_kind_check
  CHECK (kind IN ('xp', 'hit_points', 'resource', 'item', 'equip', 'level', 'death'));
