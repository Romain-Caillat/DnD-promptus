-- session/invite-and-join — players join a campaign with a link, no
-- account; their character slot.
--
-- A player is a nickname and a device: the device holds a 256-bit
-- random token (an HttpOnly cookie scoped to the campaign) and the
-- server keeps only its SHA-256 (`auth::tokens`), like GM sessions. A
-- database dump yields no usable token (`MEMORY.md` §3).

-- The campaign's invitation link. One active link per campaign: minting
-- a new one deletes the old one, so a leaked link dies when the GM
-- regenerates it. Only the hash of the code is stored; the GM sees the
-- code once, when it is minted. Players who already joined keep their
-- place: the link only opens the door, the device token is the key.
CREATE TABLE campaign_invites (
  campaign_id UUID PRIMARY KEY REFERENCES campaigns(id) ON DELETE CASCADE,
  token_hash  TEXT NOT NULL UNIQUE,
  created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
  expires_at  TIMESTAMPTZ NOT NULL
);

-- Who joined a campaign. A spectator watches (what the shared screen
-- shows) and has no character.
CREATE TABLE players (
  id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  campaign_id  UUID NOT NULL REFERENCES campaigns(id) ON DELETE CASCADE,
  nickname     TEXT NOT NULL,
  role         TEXT NOT NULL CHECK (role IN ('player', 'spectator')),
  token_hash   TEXT NOT NULL UNIQUE,
  created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
  last_seen_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX players_campaign_idx ON players (campaign_id, created_at);
-- Two « Marc » at one table would leave the GM guessing who is who.
CREATE UNIQUE INDEX players_nickname_idx ON players (campaign_id, lower(nickname));

-- A player's character, from the first draft to the GM's validation
-- (`session/validate-characters`). `sheet` is what the player wrote
-- (`players::CharacterSheet`): it is sent back to that player whole, so
-- nothing GM-only may ever be stored in it. `gm_note` is the GM's word
-- when a sheet is returned, written for the player.
CREATE TABLE characters (
  id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  campaign_id UUID NOT NULL REFERENCES campaigns(id) ON DELETE CASCADE,
  player_id   UUID NOT NULL REFERENCES players(id) ON DELETE CASCADE,
  status      TEXT NOT NULL DEFAULT 'draft'
              CHECK (status IN ('draft', 'submitted', 'validated', 'returned')),
  sheet       JSONB NOT NULL DEFAULT '{}'::jsonb,
  gm_note     TEXT,
  created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- One character per player for now; a dead character's player making a
-- new one (`player/face-death`) will relax this.
CREATE UNIQUE INDEX characters_player_idx ON characters (player_id);
CREATE INDEX characters_campaign_idx ON characters (campaign_id);

CREATE TRIGGER characters_set_updated_at
  BEFORE UPDATE ON characters
  FOR EACH ROW EXECUTE FUNCTION set_updated_at();
