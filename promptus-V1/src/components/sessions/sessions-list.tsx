"use client";

import Link from "next/link";
import { useQuery } from "@tanstack/react-query";
import { Card, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import type { Session } from "@/lib/db/schema";
import { PHASE_LABELS } from "@/lib/engine/catalog";

export function SessionsList({ campaignId }: { campaignId: string }) {
  const { data, isLoading } = useQuery({
    queryKey: ["sessions", campaignId],
    queryFn: async () => {
      const res = await fetch(`/api/campaigns/${campaignId}/sessions`);
      if (!res.ok) throw new Error("Impossible de charger les sessions");
      return res.json() as Promise<{ sessions: Session[] }>;
    },
  });

  if (isLoading) {
    return <p className="text-sm text-muted-foreground">Chargement des sessions…</p>;
  }
  const list = data?.sessions ?? [];
  if (list.length === 0) {
    return (
      <p className="text-sm text-muted-foreground italic">
        Aucune session. Lancez-en une ci-dessus pour ouvrir le poste de pilotage.
      </p>
    );
  }
  return (
    <div className="grid gap-3 md:grid-cols-2">
      {list.map((s) => (
        <Link key={s.id} href={`/sessions/${s.id}`}>
          <Card className="hover:border-primary transition-colors h-full">
            <CardHeader>
              <div className="flex items-start justify-between gap-2">
                <CardTitle className="text-lg">{s.name}</CardTitle>
                <Badge variant={s.endedAt ? "outline" : "default"} >
                  {s.endedAt ? "terminée" : PHASE_LABELS[s.currentPhase]}
                </Badge>
              </div>
              <CardDescription>
                {new Date(s.startedAt).toLocaleString("fr-FR")}
                {s.combatRound > 0 ? ` · round ${s.combatRound}` : ""}
                {s.recap ? (s.recap.status === "published" ? " · récap publié" : " · récap à relire") : ""}
              </CardDescription>
            </CardHeader>
          </Card>
        </Link>
      ))}
    </div>
  );
}
