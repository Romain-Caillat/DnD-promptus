-- session/pair-shared-screen, tv/show-evening — the shared screen: a TV
-- in the living room, or a window the GM shares on Discord.
--
-- A screen has no account, like a player: opening `/tv` leaves a random
-- token on the device (an HttpOnly cookie on `/api/tv`) and the server
-- keeps its hash only. Until a GM pairs it, the screen shows a short
-- `code` that expires; pairing binds it to one campaign for good (it
-- reconnects by itself at the next evening) until the GM forgets it,
-- which deletes the row and so the token. A `window` is paired at birth
-- by the GM's own browser: no code.
CREATE TABLE shared_screens (
  id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  token_hash      TEXT NOT NULL UNIQUE,
  kind            TEXT NOT NULL CHECK (kind IN ('tv', 'window')),
  -- Shown while waiting: four characters, unique among live codes.
  code            TEXT,
  code_expires_at TIMESTAMPTZ,
  campaign_id     UUID REFERENCES campaigns(id) ON DELETE CASCADE,
  paired_at       TIMESTAMPTZ,
  created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
  CHECK ((campaign_id IS NULL) = (paired_at IS NULL)),
  CHECK (campaign_id IS NULL OR code IS NULL)
);

CREATE UNIQUE INDEX shared_screens_code_idx ON shared_screens (code) WHERE code IS NOT NULL;
CREATE INDEX shared_screens_campaign_idx ON shared_screens (campaign_id) WHERE campaign_id IS NOT NULL;

-- What the GM lets the shared screens of a campaign show. It can only
-- narrow the players' projection, never widen it; a campaign without a
-- row shows everything the projection allows.
CREATE TABLE shared_screen_settings (
  campaign_id UUID PRIMARY KEY REFERENCES campaigns(id) ON DELETE CASCADE,
  -- `{ scene, map, party, moments }` booleans (`screens::Shows`).
  shows       JSONB NOT NULL,
  updated_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TRIGGER shared_screen_settings_set_updated_at
  BEFORE UPDATE ON shared_screen_settings
  FOR EACH ROW EXECUTE FUNCTION set_updated_at();
