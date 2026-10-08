-- player/play-between-sessions — what a player does between two
-- evenings: spend the upgrade points the rules gave them, and read what
-- the last session brought them.

-- Upgrade points spent, by ability id of the rule system:
-- `{ "FOR": 2, "CHA": 1 }`. Each adds 1 to that score. The points
-- earned stay derived from the XP (`players::play::combatant`); the
-- points left are those earned minus these, never below zero (the GM
-- may take XP back after a point was spent).
ALTER TABLE character_play ADD COLUMN upgrades JSONB NOT NULL DEFAULT '{}'::jsonb;

-- The player spending a point is logged like any other change.
-- The list keeps every kind of the earlier migrations (024 added `level`
-- and `death`): the last migration to run wins.
ALTER TABLE play_adjustments DROP CONSTRAINT play_adjustments_kind_check;
ALTER TABLE play_adjustments ADD CONSTRAINT play_adjustments_kind_check
  CHECK (kind IN ('xp', 'hit_points', 'resource', 'item', 'equip', 'level', 'death', 'upgrade'));

-- The character a shared journal line is about (loot handed to them, a
-- purchase, a gift), so the end-of-evening screen shows a player what
-- *they* got without parsing the line's text.
ALTER TABLE table_journal
  ADD COLUMN character_id UUID REFERENCES characters(id) ON DELETE SET NULL;

CREATE INDEX table_journal_character_idx ON table_journal (session_id, character_id)
  WHERE character_id IS NOT NULL;
