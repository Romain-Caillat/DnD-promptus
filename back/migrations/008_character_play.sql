-- gm/adjust-sheets-fast, player/read-sheet-and-journal — a validated
-- character's live play state, and the history of what changed it.
--
-- `characters.sheet` stays what the player wrote and the GM reviewed
-- (its snapshot `reviewed_sheet` drives the review's diff). What moves
-- at the table — hit points lost, XP earned, the purse, the bag — lives
-- here, held by the server: the GM adjusts it, the player reads it and
-- only chooses what they carry (`equipped`). Everything the rules
-- derive (maximum hit points, level, armour class, cards) is computed
-- from the sheet and this state, never stored (`players::play`).
--
-- A character with no row plays from its starting state: no damage, no
-- XP, the rules' starting resources and the class's starting items. The
-- first write stores that state with the change applied.
CREATE TABLE character_play (
  character_id UUID PRIMARY KEY REFERENCES characters(id) ON DELETE CASCADE,
  campaign_id  UUID NOT NULL REFERENCES campaigns(id) ON DELETE CASCADE,
  -- Every XP point ever earned: the rules make the level, the bar and
  -- the upgrade points of it.
  total_xp     INT NOT NULL DEFAULT 0 CHECK (total_xp >= 0),
  -- Hit points lost, not hit points left: when the rules raise the
  -- maximum (a level), the character keeps their wounds, not their total.
  damage       INT NOT NULL DEFAULT 0 CHECK (damage >= 0),
  -- Amount by resource id of the rule system (`or`); a missing id holds
  -- the rules' starting amount.
  resources    JSONB NOT NULL DEFAULT '{}'::jsonb,
  -- `[{ key, item?, name, description, qty, equipped }]`
  -- (`players::play::InventoryEntry`): a rule system item by id, or one
  -- the GM named. Shown to the player whole.
  inventory    JSONB NOT NULL DEFAULT '[]'::jsonb,
  created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at   TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX character_play_campaign_idx ON character_play (campaign_id);

CREATE TRIGGER character_play_set_updated_at
  BEFORE UPDATE ON character_play
  FOR EACH ROW EXECUTE FUNCTION set_updated_at();

-- Every change to a play state, append-only: who (the GM, or the player
-- equipping an item), what, before and after, when. There is no session
-- entity yet (`session/end-session`), so the history is the campaign's;
-- a session will be a time range over it. GM-side: no player route reads
-- this table.
CREATE TABLE play_adjustments (
  id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  campaign_id  UUID NOT NULL REFERENCES campaigns(id) ON DELETE CASCADE,
  character_id UUID NOT NULL REFERENCES characters(id) ON DELETE CASCADE,
  actor        TEXT NOT NULL CHECK (actor IN ('gm', 'player')),
  kind         TEXT NOT NULL
               CHECK (kind IN ('xp', 'hit_points', 'resource', 'item', 'equip')),
  -- The resource's or the item's name at the time, for `resource`,
  -- `item` and `equip`.
  label        TEXT,
  before_value INT NOT NULL,
  after_value  INT NOT NULL,
  created_at   TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX play_adjustments_campaign_idx ON play_adjustments (campaign_id, created_at DESC);
