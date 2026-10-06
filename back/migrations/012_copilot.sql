-- copilot/draft-narration, copilot/propose-adversary-turns — what the
-- co-GM wrote, kept as a draft until the GM shows it or drops it
-- (`MEMORY.md` §3: nothing reaches the players without the GM).
--
-- `answer` is the sanitized answer (`copilot::Answer`): invented ids
-- already dropped. Only `show` writes to the shared journal, with the
-- text the GM edited, never the draft as such.
CREATE TABLE copilot_drafts (
  id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  campaign_id UUID NOT NULL REFERENCES campaigns(id) ON DELETE CASCADE,
  session_id  UUID REFERENCES game_sessions(id) ON DELETE CASCADE,
  kind        TEXT NOT NULL CHECK (kind IN ('describe', 'npc', 'consequence', 'next', 'free')),
  prompt      TEXT NOT NULL DEFAULT '',
  answer      JSONB NOT NULL,
  status      TEXT NOT NULL DEFAULT 'draft' CHECK (status IN ('draft', 'shown', 'dismissed')),
  created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
  decided_at  TIMESTAMPTZ
);

CREATE INDEX copilot_drafts_session_idx ON copilot_drafts (session_id, created_at);
