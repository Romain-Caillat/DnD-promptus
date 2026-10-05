-- session/stream-live-changes — "this changed" counters.
--
-- One row per (campaign, topic): `world`, `story`, `character:<id>`…
-- Every write that players or the GM may be looking at bumps its topic's
-- version in the same transaction, then `pg_notify('promptus_live', …)`
-- tells the server's listener, which relays `{topic, version}` to the
-- campaign's sockets (`back/src/live`). The counters are what lets a
-- client that was cut off know what it missed: on (re)connection the
-- server sends the current versions and the client refetches every topic
-- whose version moved. They carry no game data (`MEMORY.md` §3).
--
-- Bumps happen under the campaign row lock (`MEMORY.md` §3), and the
-- upsert locks the counter row besides, so versions strictly increase
-- in commit order.

CREATE TABLE live_versions (
  campaign_id UUID NOT NULL REFERENCES campaigns(id) ON DELETE CASCADE,
  topic       TEXT NOT NULL,
  version     BIGINT NOT NULL,
  PRIMARY KEY (campaign_id, topic)
);
