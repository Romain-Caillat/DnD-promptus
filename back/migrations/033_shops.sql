-- player/buy-and-trade — a shop the GM opens (Dents-de-Fer's black
-- market at Kerjean, a Cephalopod trader), what players buy there, and
-- the one haggle each character may try.
--
-- Every write takes the campaign row lock (`MEMORY.md` §3) and touches
-- the `session` live topic; the purse and the bag move through
-- `players::play` like every other change of a character in play.

CREATE TABLE shops (
  id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  campaign_id UUID NOT NULL REFERENCES campaigns(id) ON DELETE CASCADE,
  -- What players read: « Le marché noir de Kerjean », « Dents-de-Fer ».
  name        TEXT NOT NULL,
  keeper      TEXT NOT NULL DEFAULT '',
  -- The story NPC who keeps it, when there is one. GM-side.
  npc         TEXT,
  -- Players see a shop only while it is open.
  open        BOOLEAN NOT NULL DEFAULT false,
  -- The rules resource prices are in (the Corsaires' `or`).
  currency    TEXT NOT NULL,
  -- `[{ key, item?, storyItem?, name, description, price, stock?, hidden }]`
  -- (`shops::Line`): a hidden line is « sous le comptoir » until the GM
  -- reveals it, or a natural 20 at haggling does.
  lines       JSONB NOT NULL DEFAULT '[]'::jsonb,
  -- `{ ability, difficulty, discountPercent, fumbleSurcharge, criticalReveals }`
  -- (`shops::Haggle`), or NULL when the keeper does not haggle.
  haggle      JSONB,
  -- Added to every price after a natural 1 at haggling (« pour le
  -- dérangement »), for everyone.
  surcharge   INT NOT NULL DEFAULT 0 CHECK (surcharge >= 0),
  created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX shops_campaign_idx ON shops (campaign_id, created_at);

CREATE TRIGGER shops_set_updated_at
  BEFORE UPDATE ON shops
  FOR EACH ROW EXECUTE FUNCTION set_updated_at();

-- One haggle per character per shop: the server's roll and what it won.
CREATE TABLE shop_haggles (
  shop_id       UUID NOT NULL REFERENCES shops(id) ON DELETE CASCADE,
  character_id  UUID NOT NULL REFERENCES characters(id) ON DELETE CASCADE,
  -- `critical_failure`, `failure`, `success`, `critical_success`.
  band          TEXT NOT NULL,
  roll          JSONB NOT NULL,
  -- A won haggle lowers the price of one purchase, of the player's
  -- choice; true until that purchase.
  discount_left BOOLEAN NOT NULL DEFAULT false,
  created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
  PRIMARY KEY (shop_id, character_id)
);
