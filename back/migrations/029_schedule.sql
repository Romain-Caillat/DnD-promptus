-- session/schedule-sessions — the GM proposes dates, the players say
-- which suit them, the GM picks one. The day before and an hour before,
-- a reminder goes to the table; a quarter of an hour before, the lobby
-- opens on its own (`back/src/schedule`).

-- A date for the next session. `proposed` while the table answers;
-- `chosen` once the GM picks it (the other proposals are `dropped`);
-- `done` once its time has passed. At most one chosen date per campaign.
CREATE TABLE session_dates (
  id               UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  campaign_id      UUID NOT NULL REFERENCES campaigns(id) ON DELETE CASCADE,
  starts_at        TIMESTAMPTZ NOT NULL,
  minutes          INTEGER NOT NULL DEFAULT 150 CHECK (minutes BETWEEN 30 AND 720),
  status           TEXT NOT NULL DEFAULT 'proposed'
                   CHECK (status IN ('proposed', 'chosen', 'dropped', 'done')),
  created_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
  chosen_at        TIMESTAMPTZ,
  -- Claimed once each by the scheduler (`UPDATE … WHERE … IS NULL`), so
  -- a restart or a second server never sends a reminder twice.
  eve_reminded_at  TIMESTAMPTZ,
  hour_reminded_at TIMESTAMPTZ,
  lobby_opened_at  TIMESTAMPTZ
);

CREATE UNIQUE INDEX session_dates_chosen_idx ON session_dates (campaign_id)
  WHERE status = 'chosen';
CREATE INDEX session_dates_campaign_idx ON session_dates (campaign_id, starts_at);

-- A player's answer for a proposed date. Spectators do not answer.
CREATE TABLE date_answers (
  date_id     UUID NOT NULL REFERENCES session_dates(id) ON DELETE CASCADE,
  player_id   UUID NOT NULL REFERENCES players(id) ON DELETE CASCADE,
  available   BOOLEAN NOT NULL,
  answered_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  PRIMARY KEY (date_id, player_id)
);

-- Where the reminders go: the table's Discord channel, through a webhook
-- the GM pastes. The URL is a secret (whoever holds it posts in the
-- channel): GM-only, never sent back whole.
CREATE TABLE campaign_reminders (
  campaign_id     UUID PRIMARY KEY REFERENCES campaigns(id) ON DELETE CASCADE,
  discord_webhook TEXT NOT NULL,
  updated_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Every message sent to the table about a date, delivered or not, for
-- the GM's screen.
CREATE TABLE reminder_log (
  id      BIGSERIAL PRIMARY KEY,
  date_id UUID NOT NULL REFERENCES session_dates(id) ON DELETE CASCADE,
  kind    TEXT NOT NULL CHECK (kind IN ('chosen', 'eve', 'hour', 'lobby')),
  ok      BOOLEAN NOT NULL,
  detail  TEXT NOT NULL DEFAULT '',
  at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX reminder_log_date_idx ON reminder_log (date_id, at);
