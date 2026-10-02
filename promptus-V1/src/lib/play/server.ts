import "server-only";
import { createHash, randomBytes } from "node:crypto";
import { and, desc, eq } from "drizzle-orm";
import { db } from "@/lib/db/client";
import {
  campaigns,
  entities,
  playerRequests,
  sessionPlayers,
  sessionState,
  sessionTimeline,
  sessions,
  type Session,
  type SessionPlayerRow,
} from "@/lib/db/schema";
import { ApiError, notFound } from "@/lib/api/errors";
import { generateId } from "@/lib/api/ids";
import { DND5E_RULESET } from "@/lib/engine/ruleset";
import { EMPTY_STORY } from "@/lib/engine/story";
import { normalizeWorld } from "@/lib/engine/world";
import type { EntityState } from "@/lib/engine/types";
import { projectPlayerView, type PlayerView } from "./projection";

export const PLAYER_TOKEN_HEADER = "x-player-token";

export function hashToken(token: string): string {
  return createHash("sha256").update(token).digest("hex");
}

export function newPlayerToken(): string {
  return randomBytes(24).toString("hex");
}

/** Code d'invitation de la session, créé à la première demande. */
export async function ensureInviteCode(sessionId: string): Promise<string> {
  const [row] = await db.select({ code: sessions.inviteCode }).from(sessions).where(eq(sessions.id, sessionId));
  if (!row) notFound("session", sessionId);
  if (row.code) return row.code;
  const code = generateId("inv").slice(4);
  await db.update(sessions).set({ inviteCode: code }).where(eq(sessions.id, sessionId));
  return code;
}

export async function sessionByInvite(code: string): Promise<Session> {
  const [row] = await db.select().from(sessions).where(eq(sessions.inviteCode, code));
  if (!row) throw new ApiError(404, "not_found", "Lien d’invitation invalide ou expiré");
  return row;
}

/** Joueur authentifié par son jeton (en-tête x-player-token), ou null. */
export async function playerFromRequest(req: Request, session: Session): Promise<SessionPlayerRow | null> {
  const token = req.headers.get(PLAYER_TOKEN_HEADER);
  if (!token) return null;
  const [p] = await db
    .select()
    .from(sessionPlayers)
    .where(and(eq(sessionPlayers.tokenHash, hashToken(token)), eq(sessionPlayers.sessionId, session.id)));
  return p ?? null;
}

export async function requirePlayer(req: Request, session: Session): Promise<SessionPlayerRow> {
  const p = await playerFromRequest(req, session);
  if (!p) throw new ApiError(401, "unauthorized", "Rejoignez d’abord la partie");
  return p;
}

// ----------------------------------------------------------------------------
// Vue joueur assemblée depuis la base
// ----------------------------------------------------------------------------


export async function loadPlayerView(session: Session, player: SessionPlayerRow): Promise<PlayerView> {
  const [campaign] = await db.select().from(campaigns).where(eq(campaigns.id, session.campaignId));
  const ents = await db.select().from(entities).where(eq(entities.campaignId, session.campaignId));
  const states = await db.select().from(sessionState).where(eq(sessionState.sessionId, session.id));
  const players = await db.select().from(sessionPlayers).where(eq(sessionPlayers.sessionId, session.id));
  const timeline = await db
    .select()
    .from(sessionTimeline)
    .where(and(eq(sessionTimeline.sessionId, session.id), eq(sessionTimeline.isPublic, true)))
    .orderBy(desc(sessionTimeline.createdAt))
    .limit(40);
  const requests = await db
    .select()
    .from(playerRequests)
    .where(eq(playerRequests.playerId, player.id))
    .orderBy(desc(playerRequests.createdAt))
    .limit(10);

  return projectPlayerView({
    session,
    campaignName: campaign?.name ?? "",
    story: campaign?.story ?? EMPTY_STORY,
    world: normalizeWorld(campaign?.worldState),
    ruleset: campaign?.ruleset ?? DND5E_RULESET,
    entities: ents.map((e) => ({
      id: e.id,
      name: e.name,
      type: e.type,
      description: e.description,
      imageUrl: e.imageUrl,
      visibility: e.visibility,
      attributes: e.attributes,
    })),
    states: states.map((s) => ({ entityId: s.entityId, currentState: s.currentState as EntityState })),
    player: { id: player.id, name: player.name, characterEntityId: player.characterEntityId },
    takenBy: Object.fromEntries(players.filter((p) => p.characterEntityId).map((p) => [p.characterEntityId!, p.name])),
    timeline: timeline.map((t) => ({ id: t.id, description: t.description, createdAt: t.createdAt.toISOString(), kind: t.kind })),
    requests: requests.map((r) => ({
      id: r.id,
      label: r.label,
      status: r.status,
      result: r.result,
      createdAt: r.createdAt.toISOString(),
    })),
  });
}
