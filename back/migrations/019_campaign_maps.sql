-- maps/edit-map-gm, maps/generate-map-llm, maps/import-image-map — the
-- maps a campaign owns, next to the worlds' maps compiled into the
-- server (`content/maps/`).
--
-- `map` is a whole `maps::Map` (format: `docs/map-format.md`), checked by
-- `Map::validate` on every write. A map reaches the table only once the
-- GM validated it (`validated_at`); a later edit withdraws the
-- validation. `backdrop` holds an imported image: decor only, the grid
-- carries the rules.
CREATE TABLE campaign_maps (
  campaign_id   UUID NOT NULL REFERENCES campaigns(id) ON DELETE CASCADE,
  id            TEXT NOT NULL,
  map           JSONB NOT NULL,
  source        TEXT NOT NULL CHECK (source IN ('editor', 'generated', 'imported')),
  -- The scene it was made for, when one.
  node          TEXT,
  backdrop      BYTEA,
  backdrop_mime TEXT,
  validated_at  TIMESTAMPTZ,
  created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
  PRIMARY KEY (campaign_id, id)
);

CREATE TRIGGER campaign_maps_updated_at
  BEFORE UPDATE ON campaign_maps
  FOR EACH ROW EXECUTE FUNCTION set_updated_at();
