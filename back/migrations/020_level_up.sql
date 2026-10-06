-- engine/level-up — what a player chooses when their character levels:
-- each level's hit die (rolled by the server, or the average), and the
-- abilities their upgrade points went to. Both are choices, not
-- derivations: the rules make the rest (maximum hit points, level,
-- upgrade points left) of them and this, never stored
-- (`players::play::combatant`).

-- Level → the hit die kept for it (`{ "2": 6, "4": 7 }`). A level
-- reached without a choice counts the die's average.
ALTER TABLE character_play
  ADD COLUMN level_hit_dice JSONB NOT NULL DEFAULT '{}'::jsonb;
-- Ability id → points spent on it (`{ "FOR": 1 }`).
ALTER TABLE character_play
  ADD COLUMN upgrades JSONB NOT NULL DEFAULT '{}'::jsonb;

-- `level`: a level's hit points taken (maximum before and after).
-- `upgrade`: an upgrade point spent (the ability's score before and
-- after).
ALTER TABLE play_adjustments DROP CONSTRAINT play_adjustments_kind_check;
ALTER TABLE play_adjustments ADD CONSTRAINT play_adjustments_kind_check
  CHECK (kind IN ('xp', 'hit_points', 'resource', 'item', 'equip', 'level', 'upgrade'));
