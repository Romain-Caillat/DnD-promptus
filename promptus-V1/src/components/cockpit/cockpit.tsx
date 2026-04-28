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
      if (!res.ok) throw new Error("Failed to load session");
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
          <Link href={`/campaigns/${campaignId}`}>Back to campaign</Link>
        </Button>
      </div>
    );
  }

  if (isLoading || !data) {
    return (
      <div className="p-8 text-center text-muted-foreground">Loading session…</div>
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
                  <Link href={`/campaigns/${campaignId}`}>← Campaign</Link>
                </Button>
                <h1 className="text-2xl font-bold">{session.name}</h1>
                <div className="flex items-center gap-2 text-xs text-muted-foreground mt-1">
                  <Badge variant="outline" className="capitalize" data-testid="phase-badge">
                    {session.currentPhase}
                  </Badge>
                  {session.combatRound > 0 ? (
                    <span>Round {session.combatRound}</span>
                  ) : (
                    <span>Pre-combat</span>
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

        {/* Hotbar */}
        <Hotbar
          phase={session.currentPhase}
          sessionId={session.id}
          campaignId={campaignId}
          participants={participants}
        />

        {/* Entity preview (reveal images on the table) */}
        <EntityPreview campaignId={campaignId} />

        {/* Participants */}
        <section className="space-y-3">
          {playerParticipants.length > 0 ? (
            <div>
              <h2 className="text-xs font-semibold uppercase tracking-wide text-muted-foreground mb-2">
                Player characters
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
                Adversaries & NPCs
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
              No participants in this session.
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
