-- campaign/import-pixellab-assets — images Claude generates at PixelLab
-- and drops into Promptus, waiting for the GM like the others.
--
-- `element` is one slot of the theme the HD-2D scene draws
-- (`<tileset>/material/<id>`, `<tileset>/wall`, `<tileset>/prop/<id>[/side|/top]`,
-- `<tileset>/facade/<kind>`, `shared::elements`); `character` is an NPC's
-- or adversary's figure, its first frame as `image` and every frame in
-- `media_frames`. `source` says who drew it, `origin` where the server
-- fetched it, `meta` its form, the generations it cost and its frames.
ALTER TABLE media_assets DROP CONSTRAINT media_assets_kind_check;
ALTER TABLE media_assets ADD CONSTRAINT media_assets_kind_check
  CHECK (kind IN ('scene', 'npc', 'adversary', 'location', 'item', 'tileset', 'intro', 'element', 'character'));

ALTER TABLE media_assets
  ADD COLUMN source TEXT NOT NULL DEFAULT 'model' CHECK (source IN ('model', 'pixellab')),
  ADD COLUMN origin TEXT,
  ADD COLUMN meta   JSONB NOT NULL DEFAULT '{}'::jsonb;

CREATE TABLE media_frames (
  asset_id  UUID NOT NULL REFERENCES media_assets(id) ON DELETE CASCADE,
  animation TEXT NOT NULL,
  direction TEXT NOT NULL,
  idx       INT  NOT NULL CHECK (idx >= 0),
  mime      TEXT NOT NULL,
  image     BYTEA NOT NULL,
  PRIMARY KEY (asset_id, animation, direction, idx)
);
