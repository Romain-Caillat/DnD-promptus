import { NextRequest, NextResponse } from "next/server";
import { eq } from "drizzle-orm";
import { z } from "zod";
import { db } from "@/lib/db/client";
import { campaigns, playerRequests } from "@/lib/db/schema";
import { badRequest, handleApiError } from "@/lib/api/errors";
import { generateId } from "@/lib/api/ids";
import { DND5E_RULESET } from "@/lib/engine/ruleset";
import { loadPlayerView, requirePlayer, sessionByInvite } from "@/lib/play/server";
import { notifySession } from "@/lib/realtime/notify";

const BodySchema = z.object({
  actionId: z.string(),
  note: z.string().trim().max(500).optional(),
  /** Attaque : arme choisie et cible visible sur la carte. */
  attackId: z.string().optional(),
  targetId: z.string().optional(),
});

/** Le joueur demande une action ; le MJ la tranche depuis le cockpit. */
export async function POST(
  req: NextRequest,
  { params }: { params: Promise<{ code: string }> },
) {
  try {
    const { code } = await params;
    const session = await sessionByInvite(code);
    const player = await requirePlayer(req, session);
    if (session.endedAt) badRequest("La session est terminée");
    if (!player.characterEntityId) badRequest("Un spectateur ne peut pas agir");
    const { actionId, note, attackId, targetId } = BodySchema.parse(await req.json());

    const [campaign] = await db.select({ ruleset: campaigns.ruleset }).from(campaigns).where(eq(campaigns.id, session.campaignId));
    const action = (campaign?.ruleset ?? DND5E_RULESET).actions.find((a) => a.id === actionId);
    if (!action) badRequest("Action inconnue");
    if (!action.phases.includes(session.currentPhase)) badRequest(`« ${action.label} » n’est pas possible dans cette phase`);

    let label = action.label;
    if (action.kind === "attack") {
      // Même vue que l'écran du joueur : la cible doit lui être visible.
      const view = await loadPlayerView(session, player);
      const attack = view.attacks.find((a) => a.id === attackId);
      const target = view.targets.find((t) => t.entityId === targetId);
      if (!attack) badRequest("Choisissez une attaque");
      if (!target) badRequest("Choisissez une cible visible sur la carte");
      const check = target.byAttack[attack.id];
      if (!check?.possible) badRequest(check?.reason ?? "Cible hors d’atteinte");
      label = `${action.label} : ${attack.name} → ${target.label}`;
    }

    const id = generateId("req");
    await db.insert(playerRequests).values({
      id,
      sessionId: session.id,
      playerId: player.id,
      actionId,
      label,
      note: note || null,
      attackId: action.kind === "attack" ? attackId : null,
      targetIds: action.kind === "attack" && targetId ? [targetId] : [],
    });
    await notifySession(session.id, ["requests"]);
    return NextResponse.json({ id }, { status: 201 });
  } catch (error) {
    return handleApiError(error);
  }
}
