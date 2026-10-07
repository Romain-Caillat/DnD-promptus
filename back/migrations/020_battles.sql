-- Ship battles (engine/support-vehicle-combat): the Corsaires' brig and
-- the Brasier's Cure-Dent against enemy ships, on a sea or space map.
--
-- The battle is the engine's own state (`vehicle::Battle`), replaced by
-- each command under the campaign lock, like a fight's. A boarding
-- pauses it (`boarding`) while the fight on the deck is played as an
-- ordinary encounter (`boarding_encounter`); the GM then resumes it.
-- One battle at a time per campaign, live or boarding.

CREATE TABLE battles (
  id                 UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  campaign_id        UUID NOT NULL REFERENCES campaigns(id) ON DELETE CASCADE,
  session_id         UUID REFERENCES game_sessions(id) ON DELETE SET NULL,
  -- The scene whose battle this is.
  node               TEXT NOT NULL,
  status             TEXT NOT NULL DEFAULT 'live'
                     CHECK (status IN ('live', 'boarding', 'ended')),
  -- Bumped by every command: a proposal names the version it was
  -- played from.
  version            INTEGER NOT NULL DEFAULT 1,
  battle             JSONB NOT NULL,
  -- The co-GM's proposed enemy turn, waiting for the GM (GM-only).
  proposal           JSONB,
  boarding_encounter UUID REFERENCES encounters(id) ON DELETE SET NULL,
  started_at         TIMESTAMPTZ NOT NULL DEFAULT now(),
  ended_at           TIMESTAMPTZ
);

CREATE UNIQUE INDEX battles_live_idx ON battles (campaign_id) WHERE status <> 'ended';

CREATE TABLE battle_events (
  battle_id  UUID NOT NULL REFERENCES battles(id) ON DELETE CASCADE,
  seq        INTEGER NOT NULL,
  event      JSONB NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  PRIMARY KEY (battle_id, seq)
);
