-- media/generate-images-and-video — an act's introduction video, drawn
-- in the background like a batch of images, and reviewed by the GM.
--
-- `intro` is a video whose subject is an act. `drawing` is an image or
-- video the model is still making: it has no bytes yet and becomes
-- `pending` (or `rejected`, with `error`) when the model answers.
-- Videos are counted in `ai_calls` as their own kind.
ALTER TABLE media_assets DROP CONSTRAINT media_assets_kind_check;
ALTER TABLE media_assets ADD CONSTRAINT media_assets_kind_check
  CHECK (kind IN ('scene', 'npc', 'adversary', 'location', 'item', 'tileset', 'intro'));

ALTER TABLE media_assets DROP CONSTRAINT media_assets_status_check;
ALTER TABLE media_assets ADD CONSTRAINT media_assets_status_check
  CHECK (status IN ('drawing', 'pending', 'approved', 'rejected'));

ALTER TABLE ai_calls DROP CONSTRAINT ai_calls_kind_check;
ALTER TABLE ai_calls ADD CONSTRAINT ai_calls_kind_check
  CHECK (kind IN ('llm', 'image', 'video'));
