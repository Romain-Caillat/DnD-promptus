"use client";

import { useEffect } from "react";
import Link from "next/link";
import { useQuery } from "@tanstack/react-query";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { useSessionStore, type ParticipantView, type SessionView } from "@/lib/stores/session-store";
import { PhaseSwitcher } from "./phase-switcher";
import { InitiativeTracker } from "./initiative-tracker";
import { ParticipantCard } from "./participant-card";
import { Hotbar } from "./hotbar";
import { TimelineFeed } from "./timeline-feed";
import { EntityPreview } from "./entity-preview";
import { AudioControls } from "./audio-controls";
import { AudioEffectListener } from "./audio-effect-listener";
import { StoryPanel } from "./story-panel";
import { PHASE_LABELS } from "@/lib/engine/catalog";

export function Cockpit({
  sessionId,
  campaignId,
}: {
  sessionId: string;
  campaignId: string;
}) {
  const { data, isLoading, error } = useQuery({
    queryKey: ["session", sessionId],
    queryFn: async () => {
      const res = await fetch(`/api/sessions/${sessionId}`);
      if (!res.ok) throw new Error("Impossible de charger la session");
      return res.json() as Promise<{
        session: SessionView;
        participants: ParticipantView[];
      }>;
    },
    refetchInterval: 5000,
  });

  const setStore = useSessionStore((s) => s.set);
  useEffect(() => {
    if (data) setStore(data.session, data.participants);
  }, [data, setStore]);

  if (error) {
    return (
      <div className="p-8 text-center">
        <p className="text-destructive">{(error as Error).message}</p>
        <Button asChild variant="outline" className="mt-4">
          <Link href={`/campaigns/${campaignId}`}>Retour à la campagne</Link>
        </Button>
      </div>
    );
  }

  if (isLoading || !data) {
    return (
      <div className="p-8 text-center text-muted-foreground">Chargement de la session…</div>
    );
  }

  const { session, participants } = data;
  const playerParticipants = participants.filter((p) => p.entity?.type === "character");
  const adversaryParticipants = participants.filter(
    (p) => p.entity?.type !== "character",
  );

  return (
    <div className="grid grid-cols-1 lg:grid-cols-[1fr_320px] gap-4 px-4 py-4 max-w-[1600px] mx-auto w-full">
      <AudioEffectListener sessionId={session.id} />
      {/* Header + main */}
      <div className="space-y-4">
        <Card>
          <CardContent className="pt-4 space-y-3">
            <div className="flex items-center justify-between gap-3 flex-wrap">
              <div>
                <Button asChild variant="ghost" size="sm" className="mb-1 -ml-2">
                  <Link href={`/campaigns/${campaignId}`}>← Campagne</Link>
                </Button>
                <h1 className="text-2xl font-bold">{session.name}</h1>
                <div className="flex items-center gap-2 text-xs text-muted-foreground mt-1">
                  <Badge variant="outline" data-testid="phase-badge">
                    {PHASE_LABELS[session.currentPhase]}
                  </Badge>
                  {session.combatRound > 0 ? (
                    <span>Round {session.combatRound}</span>
                  ) : (
                    <span>Hors combat</span>
                  )}
                </div>
              </div>
              <PhaseSwitcher
                sessionId={session.id}
                currentPhase={session.currentPhase}
              />
            </div>
          </CardContent>
        </Card>

        {/* Scénario : la narration d’abord */}
        <StoryPanel sessionId={session.id} campaignId={campaignId} participants={participants} />

        {/* Hotbar */}
        <Hotbar
          phase={session.currentPhase}
          sessionId={session.id}
          campaignId={campaignId}
          participants={participants}
        />

        {/* Révélation d’images */}
        <EntityPreview campaignId={campaignId} />

        {/* Participants */}
        <section className="space-y-3">
          {playerParticipants.length > 0 ? (
            <div>
              <h2 className="text-xs font-semibold uppercase tracking-wide text-muted-foreground mb-2">
                Personnages joueurs
              </h2>
              <div className="grid gap-3 md:grid-cols-2">
                {playerParticipants.map((p) => (
                  <ParticipantCard
                    key={p.state.entityId}
                    sessionId={session.id}
                    participant={p}
                  />
                ))}
              </div>
            </div>
          ) : null}
          {adversaryParticipants.length > 0 ? (
            <div>
              <h2 className="text-xs font-semibold uppercase tracking-wide text-muted-foreground mb-2 mt-3">
                Adversaires et PNJ
              </h2>
              <div className="grid gap-3 md:grid-cols-2">
                {adversaryParticipants.map((p) => (
                  <ParticipantCard
                    key={p.state.entityId}
                    sessionId={session.id}
                    participant={p}
                  />
                ))}
              </div>
            </div>
          ) : null}
          {participants.length === 0 ? (
            <p className="text-sm text-muted-foreground italic">
              Aucun participant dans cette session.
            </p>
          ) : null}
        </section>
      </div>

      {/* Sidebar: initiative + audio + timeline */}
      <aside className="space-y-4">
        <InitiativeTracker session={session} participants={participants} />
        <AudioControls />
        <TimelineFeed sessionId={session.id} />
      </aside>
    </div>
  );
}
