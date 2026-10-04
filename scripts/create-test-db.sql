-- Integration tests run against their own database so they never touch
-- dev data (TEST_DATABASE_URL in .env.example). Executed by the Postgres
-- image on first start of a fresh volume; `bun run db:reset` replays it.
CREATE DATABASE promptus_test OWNER promptus;
