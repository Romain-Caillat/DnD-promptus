-- session/pair-shared-screen, gm/launch-session — a TV (or a window the
-- GM shares on Discord) joins a table as a spectator seat marked
-- `screen`: it sees exactly what the player projection gives a
-- spectator, nothing more.

ALTER TABLE players ADD COLUMN screen BOOLEAN NOT NULL DEFAULT false;
-- A screen only ever watches.
ALTER TABLE players ADD CONSTRAINT players_screen_check CHECK (NOT screen OR role = 'spectator');

-- A TV waiting to be paired shows `code`; it polls with the secret it
-- alone holds. Once the GM types the code, the seat's device token waits
-- here until the TV collects it, then the row goes. Unpaired rows older
-- than a quarter of an hour are dropped.
CREATE TABLE tv_pairings (
  secret_hash  TEXT PRIMARY KEY,
  code         TEXT NOT NULL CHECK (code ~ '^[A-Z0-9]{4}$'),
  campaign_id  UUID REFERENCES campaigns(id) ON DELETE CASCADE,
  player_token TEXT,
  created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
  CHECK ((campaign_id IS NULL) = (player_token IS NULL))
);

CREATE UNIQUE INDEX tv_pairings_code_idx ON tv_pairings (code) WHERE campaign_id IS NULL;

-- « Précédemment… » read line by line at the launch: how many lines the
-- table sees now; NULL when no reading is under way.
ALTER TABLE game_sessions ADD COLUMN reading_line INT CHECK (reading_line >= 0);
