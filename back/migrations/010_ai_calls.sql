-- ai/count-ai-calls — every AI call a campaign makes, with what it cost.
--
-- One row per call that reached a provider, successful or not (a failed
-- call may still be billed). The campaign's spending is the sum of
-- `cost_micros` (millionths of a US dollar, the unit OpenRouter reports
-- with enough precision); its budget is `campaigns.ai_budget_cents`
-- (migration 006). `ai::Ledger` estimates a batch before sending it and
-- refuses the whole batch when spending plus the estimate would pass the
-- budget: a refused batch leaves no row here because no call left.
--
-- `template` is the versioned prompt the call was built from
-- (`copilot@1`), so a change of prompt shows in the history.

CREATE TABLE ai_calls (
  id                UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  campaign_id       UUID NOT NULL REFERENCES campaigns(id) ON DELETE CASCADE,
  kind              TEXT NOT NULL CHECK (kind IN ('llm', 'image')),
  -- What the call was for (`copilot.describe`, `media.npc`…).
  purpose           TEXT NOT NULL,
  template          TEXT,
  provider          TEXT NOT NULL,
  model             TEXT NOT NULL,
  prompt_tokens     INT NOT NULL DEFAULT 0 CHECK (prompt_tokens >= 0),
  completion_tokens INT NOT NULL DEFAULT 0 CHECK (completion_tokens >= 0),
  cost_micros       BIGINT NOT NULL DEFAULT 0 CHECK (cost_micros >= 0),
  -- `NULL` for a call that answered; the provider's error otherwise.
  error             TEXT,
  -- false while the call is in flight: `cost_micros` holds its estimate.
  settled           BOOLEAN NOT NULL DEFAULT false,
  created_at        TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX ai_calls_campaign_idx ON ai_calls (campaign_id, created_at DESC);
