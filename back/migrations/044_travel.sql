-- maps/travel-hex-world — the party on a world map in hexes, and its
-- journey from one place to another.
--
-- One row per campaign and world map: the party is one token there
-- (`party`, `promptus_shared::travel::Party`: its hex, the hexes it has
-- seen, the day and portion, its supplies, the way it follows), kept
-- when the table goes down to a place map and comes back. `journey` is
-- the trip under way (`travel::Journey`): the routes proposed and the
-- players' votes, the route chosen, the events kept, the group check
-- and the night's watch. `proposal` holds the events drawn for the GM
-- after a portion: GM-only until one is kept.
--
-- Every write takes the campaign lock (`MEMORY.md` §3) and touches the
-- `map` live topic; the board's copy of the world map (`map_states`)
-- follows the party in the same transaction.
CREATE TABLE travels (
  campaign_id UUID NOT NULL REFERENCES campaigns(id) ON DELETE CASCADE,
  map_id      TEXT NOT NULL,
  party       JSONB NOT NULL,
  journey     JSONB,
  proposal    JSONB,
  -- Bumped by every write.
  version     INTEGER NOT NULL DEFAULT 1,
  updated_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
  PRIMARY KEY (campaign_id, map_id)
);
