-- ai/generate-campaign — a campaign generated from the GM's pitch, in
-- the background, step after step, followed live on the GM's screen.
--
-- The result is a draft (`draft`, a whole `story::Campaign`): nothing
-- reaches the campaign until the GM applies it, and an applied draft
-- still has to be reviewed and validated (`campaign/review-story-graph`)
-- before a session opens.
--
-- `steps`: the pipeline's steps and where each stands. `pruned`: what
-- the model invented and the server removed (`story::prune`);
-- `dropped`: the repair edits refused (`story::edit::sanitize`).
-- `cost_micros`: what the job's calls cost (each is also in `ai_calls`).
-- `error`: a machine code (`AI_BUDGET_EXCEEDED`, `AI_UNAVAILABLE`,
-- `AI_OUTPUT_INVALID`…) and `detail` the text for the GM. A job still `running` long after its last step was interrupted
-- (the server restarted): it reads as failed.
CREATE TABLE generation_jobs (
  id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  campaign_id UUID NOT NULL REFERENCES campaigns(id) ON DELETE CASCADE,
  status      TEXT NOT NULL DEFAULT 'running'
              CHECK (status IN ('running', 'succeeded', 'failed', 'applied')),
  input       JSONB NOT NULL,
  steps       JSONB NOT NULL,
  draft       JSONB,
  pruned      JSONB NOT NULL DEFAULT '[]',
  repairs     INT NOT NULL DEFAULT 0 CHECK (repairs >= 0),
  dropped     INT NOT NULL DEFAULT 0 CHECK (dropped >= 0),
  cost_micros BIGINT NOT NULL DEFAULT 0 CHECK (cost_micros >= 0),
  error       TEXT,
  detail      TEXT,
  created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX generation_jobs_campaign_idx ON generation_jobs (campaign_id, created_at DESC);
-- One job running per campaign.
CREATE UNIQUE INDEX generation_jobs_one_running
  ON generation_jobs (campaign_id) WHERE status = 'running';

CREATE TRIGGER generation_jobs_set_updated_at
  BEFORE UPDATE ON generation_jobs
  FOR EACH ROW EXECUTE FUNCTION set_updated_at();
