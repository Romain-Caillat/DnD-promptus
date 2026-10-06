-- campaign/edit-rule-system — the GM's own versions of a campaign's
-- rule system.
--
-- A campaign starts on a preset compiled into the server
-- (`content/rules/<id>/v<n>.yaml`). Its first edit copies the version it
-- plays into a draft numbered one above; the GM edits the draft as
-- often as they like (the lint and the simulation run on every save),
-- then locks it. A locked version never changes again: the campaign
-- moves to the newest locked version when its next session opens
-- (`evening::session::open`), so players read « what changed » before
-- they play, never mid-game (`MEMORY.md` §6).
--
-- `system` is the YAML the GM wrote, kept as text: the rules engine
-- reads it with the same loader as the presets (`RuleSystem::from_yaml`),
-- and a locked version's text is what every later read parses.

CREATE TABLE rule_versions (
  campaign_id UUID NOT NULL REFERENCES campaigns(id) ON DELETE CASCADE,
  version     INTEGER NOT NULL CHECK (version >= 1),
  system      TEXT NOT NULL,
  -- What the GM says this version changes, in their words.
  note        TEXT NOT NULL DEFAULT '',
  -- NULL while it is the draft.
  locked_at   TIMESTAMPTZ,
  created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
  PRIMARY KEY (campaign_id, version)
);

-- One draft at a time per campaign.
CREATE UNIQUE INDEX rule_versions_one_draft
  ON rule_versions (campaign_id) WHERE locked_at IS NULL;

CREATE TRIGGER rule_versions_set_updated_at
  BEFORE UPDATE ON rule_versions
  FOR EACH ROW EXECUTE FUNCTION set_updated_at();
