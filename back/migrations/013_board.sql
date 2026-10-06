-- The grid at the table (phase 3): the map shown, its fog and tokens
-- (player/explore-map, maps/reveal-fog-and-hidden), and the fights
-- (gm/run-combat, player/fight-turn, copilot/propose-adversary-turns).
--
-- The map is stored whole as it stands (doors opened, layers revealed,
-- ambience changed): the players' copy is cut from it on every read
-- (`Map::project`, then `Map::fogged` with `revealed`). The fight is the
-- engine's own state (`combat::fight::Fight`), replaced by each command
-- under the campaign lock.

CREATE TABLE map_states (
  campaign_id UUID PRIMARY KEY REFERENCES campaigns(id) ON DELETE CASCADE,
  map_id      TEXT NOT NULL,
  map         JSONB NOT NULL,
  fog         BOOLEAN NOT NULL DEFAULT true,
  -- Cells out of the fog, as [[x, y], …].
  revealed    JSONB NOT NULL DEFAULT '[]',
  -- `board::Token`s: who stands where outside a fight.
  tokens      JSONB NOT NULL DEFAULT '[]',
  updated_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE encounters (
  id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  campaign_id UUID NOT NULL REFERENCES campaigns(id) ON DELETE CASCADE,
  session_id  UUID REFERENCES game_sessions(id) ON DELETE SET NULL,
  -- The scene whose encounter this is.
  node        TEXT NOT NULL,
  status      TEXT NOT NULL DEFAULT 'live' CHECK (status IN ('live', 'ended')),
  -- Bumped by every command: a proposal of the co-GM names the version
  -- it was played from.
  version     INTEGER NOT NULL DEFAULT 1,
  fight       JSONB NOT NULL,
  -- The co-GM's proposed adversary turn, waiting for the GM (GM-only).
  proposal    JSONB,
  -- The scene's loot and who received each line.
  loot        JSONB NOT NULL DEFAULT '[]',
  started_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
  ended_at    TIMESTAMPTZ
);

CREATE UNIQUE INDEX encounters_live_idx ON encounters (campaign_id) WHERE status = 'live';

CREATE TABLE encounter_events (
  encounter_id UUID NOT NULL REFERENCES encounters(id) ON DELETE CASCADE,
  seq          INTEGER NOT NULL,
  event        JSONB NOT NULL,
  created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
  PRIMARY KEY (encounter_id, seq)
);
