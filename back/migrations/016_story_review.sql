-- campaign/review-story-graph — the GM reviews a campaign, works it
-- with the co-GM, and validates it: only a validated campaign opens a
-- session.
--
-- `validated_at`: when the GM last declared the campaign playable.
-- Campaigns already stored were imported or created before validation
-- existed and have been played: they count as validated.
ALTER TABLE campaigns ADD COLUMN validated_at TIMESTAMPTZ;
UPDATE campaigns SET validated_at = now();

-- The co-GM's workshop: what the GM asked, what the co-GM answered and
-- the edits it proposes (`story::edit::Edit`, invented ids already
-- dropped). Nothing changes in the campaign until the GM accepts.
CREATE TABLE story_proposals (
  id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  campaign_id UUID NOT NULL REFERENCES campaigns(id) ON DELETE CASCADE,
  prompt      TEXT NOT NULL,
  reply       TEXT NOT NULL DEFAULT '',
  edits       JSONB NOT NULL,
  dropped     INT NOT NULL DEFAULT 0 CHECK (dropped >= 0),
  status      TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'accepted', 'rejected')),
  created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
  decided_at  TIMESTAMPTZ
);

CREATE INDEX story_proposals_campaign_idx ON story_proposals (campaign_id, created_at DESC);
