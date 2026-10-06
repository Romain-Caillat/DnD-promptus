-- The evening (phase 3): sessions and their lobby, the players' requests
-- and the checks they lead to, what the table knows, the rulings the GM
-- gave, and the players' word at the end.
--
-- Every write here happens under the campaign row lock (`MEMORY.md` §3)
-- and touches the `session` live topic, so the GM screen and the phones
-- refetch through the API (`back/src/evening`).

-- session/open-lobby, session/end-session — one evening of play.
-- A campaign has at most one session not yet ended: the lobby opens it,
-- the GM starts it, « Terminer la session » ends it. Ending stores the
-- world as it stands (`world_at_end`): the next session opens on it, and
-- the chronicle is the list of ended sessions with their recaps.
CREATE TABLE game_sessions (
  id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  campaign_id  UUID NOT NULL REFERENCES campaigns(id) ON DELETE CASCADE,
  number       INTEGER NOT NULL CHECK (number >= 1),
  status       TEXT NOT NULL DEFAULT 'lobby' CHECK (status IN ('lobby', 'live', 'ended')),
  opened_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
  started_at   TIMESTAMPTZ,
  ended_at     TIMESTAMPTZ,
  -- The GM's recap: GM-only.
  recap        TEXT NOT NULL DEFAULT '',
  -- « Précédemment… », read to the players: player text once published.
  previously   TEXT NOT NULL DEFAULT '',
  world_at_end JSONB,
  -- session/collect-player-feedback: what the GM changes after reading
  -- the players' answers. GM-only.
  gm_changes   TEXT NOT NULL DEFAULT '',
  -- media/play-youtube-music: the track playing for everyone, and when
  -- it started (server time), so every phone seeks to the same second.
  -- `{ url, title, mood, startedAt }`, or NULL for silence.
  music        JSONB,
  -- What the next scenes need that the table did not know when the
  -- session ended (`evening::knowledge::gaps`): a feedback measure.
  gaps_at_end  JSONB,
  UNIQUE (campaign_id, number)
);

CREATE UNIQUE INDEX game_sessions_open_idx ON game_sessions (campaign_id)
  WHERE status <> 'ended';

-- Who said they are here, in the lobby: seen on the live channel, the
-- sound tested, and from where they play.
CREATE TABLE session_attendance (
  session_id UUID NOT NULL REFERENCES game_sessions(id) ON DELETE CASCADE,
  player_id  UUID NOT NULL REFERENCES players(id) ON DELETE CASCADE,
  sound_ok   BOOLEAN NOT NULL DEFAULT false,
  remote     BOOLEAN NOT NULL DEFAULT true,
  joined_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
  PRIMARY KEY (session_id, player_id)
);

-- session/drive-scenes — what a player asks the GM: an action card of
-- the rules, or « Autre… » in their own words. The GM accepts, refuses
-- with a reason, or asks for a check (ability and difficulty); the
-- player then rolls, and the server rolls (`roll` is the breakdown).
-- A player may contest a ruling: a measure for the feedback screen.
CREATE TABLE player_requests (
  id               UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  campaign_id      UUID NOT NULL REFERENCES campaigns(id) ON DELETE CASCADE,
  session_id       UUID NOT NULL REFERENCES game_sessions(id) ON DELETE CASCADE,
  player_id        UUID NOT NULL REFERENCES players(id) ON DELETE CASCADE,
  character_id     UUID REFERENCES characters(id) ON DELETE SET NULL,
  -- An action of the rule system, or NULL for « Autre… ».
  action_id        TEXT,
  text             TEXT NOT NULL DEFAULT '',
  status           TEXT NOT NULL DEFAULT 'pending'
                   CHECK (status IN ('pending', 'accepted', 'refused', 'check', 'rolled', 'withdrawn')),
  -- Written for the player.
  gm_reason        TEXT,
  check_ability    TEXT,
  check_difficulty INTEGER,
  -- The difficulty's name in the rules (« Moyen »), when it has one.
  check_label      TEXT,
  roll             JSONB,
  contested        BOOLEAN NOT NULL DEFAULT false,
  created_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
  decided_at       TIMESTAMPTZ,
  rolled_at        TIMESTAMPTZ
);

CREATE INDEX player_requests_session_idx ON player_requests (session_id, created_at);

-- session/track-table-knowledge — what the table knows, in the order it
-- learnt it. `shared` lines are the players' journal; the others are the
-- GM's (a hidden reveal, a note). `ref` is the story id it is about
-- (a clue, an NPC, a node), so the GM finds « what they know of Morel »
-- in one gesture.
CREATE TABLE table_journal (
  id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  campaign_id UUID NOT NULL REFERENCES campaigns(id) ON DELETE CASCADE,
  session_id  UUID REFERENCES game_sessions(id) ON DELETE SET NULL,
  kind        TEXT NOT NULL CHECK (kind IN (
                'scene', 'clue', 'npc', 'promise', 'debt', 'item', 'note',
                'roll', 'narration', 'fight', 'loot')),
  ref         TEXT,
  text        TEXT NOT NULL,
  shared      BOOLEAN NOT NULL DEFAULT true,
  created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX table_journal_campaign_idx ON table_journal (campaign_id, created_at);
CREATE INDEX table_journal_ref_idx ON table_journal (campaign_id, ref) WHERE ref IS NOT NULL;

-- The rulings the GM gives in play (« escalader le mât : DEX 10 »),
-- kept so the same situation gets the same difficulty next time.
-- GM-side.
CREATE TABLE rulings (
  id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  campaign_id UUID NOT NULL REFERENCES campaigns(id) ON DELETE CASCADE,
  session_id  UUID REFERENCES game_sessions(id) ON DELETE SET NULL,
  situation   TEXT NOT NULL,
  ability     TEXT NOT NULL,
  difficulty  INTEGER NOT NULL,
  created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX rulings_campaign_idx ON rulings (campaign_id, created_at DESC);

-- session/collect-player-feedback — thirty seconds on the phone at the
-- end of the evening. Each answer: yes, partly, no.
CREATE TABLE session_feedback (
  session_id  UUID NOT NULL REFERENCES game_sessions(id) ON DELETE CASCADE,
  player_id   UUID NOT NULL REFERENCES players(id) ON DELETE CASCADE,
  rules_clear TEXT NOT NULL CHECK (rules_clear IN ('yes', 'partly', 'no')),
  had_moment  TEXT NOT NULL CHECK (had_moment IN ('yes', 'partly', 'no')),
  knows_next  TEXT NOT NULL CHECK (knows_next IN ('yes', 'partly', 'no')),
  comment     TEXT NOT NULL DEFAULT '',
  created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
  PRIMARY KEY (session_id, player_id)
);

-- gm/balance-spotlight, session/collect-player-feedback — each moment a
-- player had in a session: a request, a roll, a move or an action in a
-- fight, or a moment the GM gave them. The spotlight reads the latest;
-- the feedback screen the longest stretch without one.
CREATE TABLE player_moments (
  id         BIGSERIAL PRIMARY KEY,
  session_id UUID NOT NULL REFERENCES game_sessions(id) ON DELETE CASCADE,
  player_id  UUID NOT NULL REFERENCES players(id) ON DELETE CASCADE,
  kind       TEXT NOT NULL CHECK (kind IN ('request', 'roll', 'move', 'fight', 'spotlight')),
  at         TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX player_moments_session_idx ON player_moments (session_id, player_id, at);

-- A hook the GM played at the table no longer waits in the spotlight list.
ALTER TABLE secret_hooks ADD COLUMN played_at TIMESTAMPTZ;

-- A result the server resolved and applied by the rules (a check's XP,
-- wounds in a fight, loot the GM validated) is logged as `rules`.
ALTER TABLE play_adjustments DROP CONSTRAINT play_adjustments_actor_check;
ALTER TABLE play_adjustments ADD CONSTRAINT play_adjustments_actor_check
  CHECK (actor IN ('gm', 'player', 'rules'));
