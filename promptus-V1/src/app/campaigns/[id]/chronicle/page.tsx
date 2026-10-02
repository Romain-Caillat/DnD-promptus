import { notFound } from "next/navigation";
import Link from "next/link";
import { asc, eq, inArray } from "drizzle-orm";
import { db } from "@/lib/db/client";
import { campaigns, sessions, sessionTimeline } from "@/lib/db/schema";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { PHASE_LABELS } from "@/lib/engine/catalog";
import { frontStep, isRevelationKnown, normalizeWorld } from "@/lib/engine/world";

export const dynamic = "force-dynamic";

/** Chronique de la campagne : sessions, récapitulatifs et journaux complets. */
export default async function ChroniclePage({ params }: { params: Promise<{ id: string }> }) {
  const { id } = await params;
  const [campaign] = await db.select().from(campaigns).where(eq(campaigns.id, id));
  if (!campaign) notFound();
  const list = await db.select().from(sessions).where(eq(sessions.campaignId, id)).orderBy(asc(sessions.startedAt));
  const entries = list.length
    ? await db
        .select()
        .from(sessionTimeline)
        .where(inArray(sessionTimeline.sessionId, list.map((s) => s.id)))
        .orderBy(asc(sessionTimeline.createdAt))
    : [];
  const story = campaign.story;
  const world = normalizeWorld(campaign.worldState);
  const known = story ? story.revelations.filter((r) => isRevelationKnown(story, world, r.id)) : [];

  return (
    <main className="flex-1 px-6 py-8 max-w-4xl mx-auto w-full space-y-6">
      <div>
        <Button asChild variant="ghost" size="sm" className="mb-2 -ml-2">
          <Link href={`/campaigns/${id}`}>← {campaign.name}</Link>
        </Button>
        <h1 className="text-3xl font-bold mb-1">Chronique</h1>
        <p className="text-muted-foreground">L’histoire de la campagne, session après session.</p>
      </div>

      {story ? (
        <Card>
          <CardContent className="pt-4 grid gap-4 md:grid-cols-2 text-sm">
            <div>
              <h2 className="text-xs uppercase tracking-wide text-muted-foreground mb-1">Ce que savent les joueurs</h2>
              {known.length ? (
                <ul className="list-disc pl-4 space-y-1">
                  {known.map((r) => (
                    <li key={r.id}>{r.statement}</li>
                  ))}
                </ul>
              ) : (
                <p className="italic text-muted-foreground">Rien encore.</p>
              )}
            </div>
            <div>
              <h2 className="text-xs uppercase tracking-wide text-muted-foreground mb-1">Menaces</h2>
              <ul className="space-y-1">
                {story.fronts.map((f) => {
                  const step = frontStep(world, f.id);
                  return (
                    <li key={f.id}>
                      {f.name} : {step + 1}/{f.steps.length}
                      {step >= 0 ? ` — ${f.steps[step]?.label}` : " (pas commencée)"}
                    </li>
                  );
                })}
              </ul>
            </div>
          </CardContent>
        </Card>
      ) : null}

      {list.length === 0 ? <p className="text-sm italic text-muted-foreground">Aucune session pour l’instant.</p> : null}
      {list.map((s, i) => {
        const log = entries.filter((e) => e.sessionId === s.id);
        return (
          <Card key={s.id} data-testid="chronicle-session">
            <CardContent className="pt-4 space-y-3">
              <div className="flex flex-wrap items-start justify-between gap-2">
                <div>
                  <p className="text-xs uppercase tracking-wide text-muted-foreground">Session {i + 1}</p>
                  <h2 className="text-lg font-semibold">
                    <Link href={`/sessions/${s.id}`} className="hover:underline">
                      {s.name}
                    </Link>
                  </h2>
                  <p className="text-xs text-muted-foreground">
                    {new Date(s.startedAt).toLocaleString("fr-FR")}
                    {s.recap ? ` · ${s.recap.facts.durationMinutes} min` : ""}
                  </p>
                </div>
                <div className="flex gap-1">
                  {s.endedAt ? <Badge variant="outline">terminée</Badge> : <Badge>{PHASE_LABELS[s.currentPhase]}</Badge>}
                  {s.recap ? (
                    <Badge variant={s.recap.status === "published" ? "default" : "secondary"}>
                      {s.recap.status === "published" ? "récap publié" : "récap en brouillon"}
                    </Badge>
                  ) : null}
                </div>
              </div>
              {s.recap ? (
                <>
                  <p className="text-sm whitespace-pre-line">{s.recap.players}</p>
                  <details className="text-sm">
                    <summary className="cursor-pointer text-muted-foreground">Notes MJ</summary>
                    <p className="mt-1 whitespace-pre-line">{s.recap.gm}</p>
                  </details>
                </>
              ) : (
                <p className="text-sm italic text-muted-foreground">
                  {s.endedAt ? "Pas de récapitulatif." : "Session en cours : terminez-la pour obtenir son récapitulatif."}
                </p>
              )}
              <details className="text-sm">
                <summary className="cursor-pointer text-muted-foreground">Journal complet ({log.length})</summary>
                <ul className="mt-2 space-y-1 text-xs">
                  {log.map((e) => (
                    <li key={e.id} className="flex gap-2">
                      <span className="text-muted-foreground shrink-0">
                        {new Date(e.createdAt).toLocaleTimeString("fr-FR", { hour: "2-digit", minute: "2-digit" })}
                      </span>
                      <span>{e.description}</span>
                      {!e.isPublic ? (
                        <Badge variant="outline" className="h-4 text-[10px]">
                          MJ
                        </Badge>
                      ) : null}
                    </li>
                  ))}
                </ul>
              </details>
            </CardContent>
          </Card>
        );
      })}
    </main>
  );
}
