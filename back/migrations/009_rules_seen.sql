-- player/read-the-rules — which version of the campaign's rules each
-- player last read.
--
-- Rule systems are versioned: a change ships as a new locked version
-- that players see before the next session (`MEMORY.md` §6). The rules
-- page compares the campaign's current version with the one recorded
-- here and lists what changed, until the player says they read it.
--
-- A seat starts with the rules of the moment it was taken: whoever joins
-- reads the whole page, not a list of changes. The trigger records that
-- version on every new seat, whatever code inserts it, so no join path
-- can forget it; the backfill gives seats taken before this migration
-- the campaign's current rules.

CREATE TABLE rules_seen (
  player_id      UUID PRIMARY KEY REFERENCES players(id) ON DELETE CASCADE,
  rule_system_id TEXT NOT NULL,
  version        INTEGER NOT NULL,
  seen_at        TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE FUNCTION rules_seen_on_join() RETURNS trigger AS $$
BEGIN
  INSERT INTO rules_seen (player_id, rule_system_id, version)
  SELECT NEW.id, c.story->'rules'->>'id', (c.story->'rules'->>'version')::int
  FROM campaigns c
  WHERE c.id = NEW.campaign_id;
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER players_rules_seen
  AFTER INSERT ON players
  FOR EACH ROW EXECUTE FUNCTION rules_seen_on_join();

INSERT INTO rules_seen (player_id, rule_system_id, version)
SELECT p.id, c.story->'rules'->>'id', (c.story->'rules'->>'version')::int
FROM players p
JOIN campaigns c ON c.id = p.campaign_id;
