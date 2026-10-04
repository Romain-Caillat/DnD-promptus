-- Shared trigger function: every table with an `updated_at` column
-- attaches it so the timestamp moves on each UPDATE without relying on
-- every query to remember it.
--
--   CREATE TRIGGER <table>_set_updated_at
--       BEFORE UPDATE ON <table>
--       FOR EACH ROW EXECUTE FUNCTION set_updated_at();
CREATE OR REPLACE FUNCTION set_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = now();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;
