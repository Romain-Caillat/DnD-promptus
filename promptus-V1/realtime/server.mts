// Serveur temps réel Promptus (WebSocket).
// Écoute les notifications Postgres (pg_notify) émises par l'API et les relaie
// aux clients abonnés à la session concernée ; gère la présence des joueurs.
// Autonome (sans alias @/, module ES) : `pnpm realtime` en dev,
// `node --experimental-strip-types realtime/server.mts` en prod.

import { createHash } from "node:crypto";
import { createServer } from "node:http";
import pg from "pg";
import { WebSocketServer, type WebSocket } from "ws";

const CHANNEL = "promptus_events";
const PORT = Number(process.env.REALTIME_PORT ?? 3001);
const DATABASE_URL = process.env.DATABASE_URL ?? "postgresql://promptus:promptus_dev@localhost:5433/promptus";

interface Member {
  ws: WebSocket;
  role: "gm" | "player";
  playerId?: string;
  name?: string;
  alive: boolean;
}

const rooms = new Map<string, Set<Member>>();
const pool = new pg.Pool({ connectionString: DATABASE_URL, max: 4 });

function send(ws: WebSocket, msg: unknown) {
  if (ws.readyState === ws.OPEN) ws.send(JSON.stringify(msg));
}

function broadcast(sessionId: string, msg: unknown) {
  for (const m of rooms.get(sessionId) ?? []) send(m.ws, msg);
}

function presence(sessionId: string) {
  const members = [...(rooms.get(sessionId) ?? [])];
  const seen = new Map<string, string>();
  for (const m of members) if (m.role === "player" && m.playerId) seen.set(m.playerId, m.name ?? "?");
  broadcast(sessionId, {
    type: "presence",
    players: [...seen].map(([playerId, name]) => ({ playerId, name })),
    gmOnline: members.some((m) => m.role === "gm"),
  });
}

async function authenticate(hello: Record<string, unknown>): Promise<{ sessionId: string; member: Omit<Member, "ws" | "alive"> } | string> {
  if (hello.type !== "hello") return "Message d’ouverture attendu";
  if (hello.role === "gm" && typeof hello.sessionId === "string") {
    const r = await pool.query("select id from sessions where id = $1", [hello.sessionId]);
    return r.rowCount ? { sessionId: hello.sessionId, member: { role: "gm" } } : "Session introuvable";
  }
  if (hello.role === "player" && typeof hello.inviteCode === "string" && typeof hello.token === "string") {
    const hash = createHash("sha256").update(hello.token).digest("hex");
    const r = await pool.query(
      `select p.id, p.name, s.id as session_id from session_players p
       join sessions s on s.id = p.session_id
       where p.token_hash = $1 and s.invite_code = $2`,
      [hash, hello.inviteCode],
    );
    if (!r.rowCount) return "Joueur inconnu pour cette session";
    const row = r.rows[0] as { id: string; name: string; session_id: string };
    return { sessionId: row.session_id, member: { role: "player", playerId: row.id, name: row.name } };
  }
  return "Identification invalide";
}

const http = createServer((req, res) => {
  res.writeHead(req.url === "/health" ? 200 : 404, { "content-type": "text/plain" });
  res.end(req.url === "/health" ? "ok" : "not found");
});
const wss = new WebSocketServer({ server: http });

wss.on("connection", (ws) => {
  let member: Member | null = null;
  let sessionId: string | null = null;

  ws.on("message", async (raw) => {
    if (member) return; // seul le premier message (identification) est attendu
    let hello: Record<string, unknown>;
    try {
      hello = JSON.parse(String(raw));
    } catch {
      send(ws, { type: "error", message: "JSON invalide" });
      return ws.close();
    }
    const auth = await authenticate(hello).catch((e) => `Erreur serveur : ${String(e)}`);
    if (typeof auth === "string") {
      send(ws, { type: "error", message: auth });
      return ws.close();
    }
    sessionId = auth.sessionId;
    member = { ws, alive: true, ...auth.member };
    if (!rooms.has(sessionId)) rooms.set(sessionId, new Set());
    rooms.get(sessionId)!.add(member);
    send(ws, { type: "ready", sessionId });
    presence(sessionId);
  });

  ws.on("pong", () => {
    if (member) member.alive = true;
  });

  ws.on("close", () => {
    if (!member || !sessionId) return;
    rooms.get(sessionId)?.delete(member);
    if (member.playerId) {
      pool.query("update session_players set last_seen_at = now() where id = $1", [member.playerId]).catch(() => {});
    }
    presence(sessionId);
  });
});

// Coupe les connexions mortes (onglet fermé sans close propre, réseau perdu).
setInterval(() => {
  for (const room of rooms.values()) {
    for (const m of room) {
      if (!m.alive) {
        m.ws.terminate();
        continue;
      }
      m.alive = false;
      m.ws.ping();
    }
  }
}, 30_000);

async function listen() {
  const client = new pg.Client({ connectionString: DATABASE_URL });
  client.on("notification", (n) => {
    try {
      const { sessionId, kinds } = JSON.parse(n.payload ?? "{}") as { sessionId: string; kinds: string[] };
      broadcast(sessionId, { type: "invalidate", kinds });
    } catch (e) {
      console.error("[realtime] bad notification", e);
    }
  });
  client.on("error", (e) => {
    console.error("[realtime] pg error, reconnecting", e.message);
    setTimeout(listen, 2000);
  });
  await client.connect();
  await client.query(`LISTEN ${CHANNEL}`);
  console.log(`[realtime] listening on Postgres channel ${CHANNEL}`);
}

listen().catch((e) => {
  console.error("[realtime] cannot connect to Postgres", e);
  process.exit(1);
});
http.listen(PORT, () => console.log(`[realtime] ws://0.0.0.0:${PORT}`));
