-- player/buy-and-trade — a shop the GM prepares and opens: its lines
-- (an item of the rules or one the GM names, a price, a stock, hidden
-- « sous le comptoir » until the GM reveals it), and whether and how
-- the table may haggle. One shop at a time is open per table.

CREATE TABLE shops (
  id                UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  campaign_id       UUID NOT NULL REFERENCES campaigns(id) ON DELETE CASCADE,
  name              TEXT NOT NULL CHECK (char_length(name) BETWEEN 1 AND 80),
  is_open           BOOLEAN NOT NULL DEFAULT false,
  -- [{ id, item?, name, description, price, stock?, hidden }]
  lines             JSONB NOT NULL DEFAULT '[]',
  -- The ability rolled to haggle, against a difficulty, and the
  -- discount (percent) a success earns. NULL: no haggling here.
  haggle_ability    TEXT,
  haggle_difficulty INT,
  haggle_discount   INT CHECK (haggle_discount BETWEEN 1 AND 90),
  -- [{ line, player, success }]: one try per player and line.
  haggles           JSONB NOT NULL DEFAULT '[]',
  created_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
  CHECK ((haggle_ability IS NULL) = (haggle_difficulty IS NULL)
         AND (haggle_ability IS NULL) = (haggle_discount IS NULL))
);

CREATE INDEX shops_campaign_idx ON shops (campaign_id);
CREATE UNIQUE INDEX shops_open_idx ON shops (campaign_id) WHERE is_open;

CREATE TRIGGER shops_updated_at BEFORE UPDATE ON shops
  FOR EACH ROW EXECUTE FUNCTION set_updated_at();
