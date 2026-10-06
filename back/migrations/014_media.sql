-- media/draw-pixel-art-assets, maps/build-tileset-packs — images drawn
-- by the image model, each one reviewed by the GM before any player may
-- see it (`MEMORY.md` §3).
--
-- `image` is NULL when the model failed (`status` is then `rejected` and
-- `error` says why). One approved image per subject: approving a new one
-- retires the previous.
CREATE TABLE media_assets (
  id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  campaign_id UUID NOT NULL REFERENCES campaigns(id) ON DELETE CASCADE,
  kind        TEXT NOT NULL CHECK (kind IN ('scene', 'npc', 'adversary', 'location', 'item', 'tileset')),
  subject     TEXT NOT NULL,
  direction   TEXT NOT NULL DEFAULT '',
  status      TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'approved', 'rejected')),
  mime        TEXT,
  image       BYTEA,
  error       TEXT,
  created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
  decided_at  TIMESTAMPTZ
);

CREATE INDEX media_assets_campaign_idx ON media_assets (campaign_id, created_at);
CREATE UNIQUE INDEX media_assets_one_approved
  ON media_assets (campaign_id, kind, subject) WHERE status = 'approved';
