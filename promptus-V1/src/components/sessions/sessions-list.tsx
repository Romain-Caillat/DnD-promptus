"use client";

import Link from "next/link";
import { useQuery } from "@tanstack/react-query";
import { Card, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import type { Session } from "@/lib/db/schema";

export function SessionsList({ campaignId }: { campaignId: string }) {
  const { data, isLoading } = useQuery({
    queryKey: ["sessions", campaignId],
    queryFn: async () => {
      const res = await fetch(`/api/campaigns/${campaignId}/sessions`);
      if (!res.ok) throw new Error("Failed to load sessions");
      return res.json() as Promise<{ sessions: Session[] }>;
    },
  });

  if (isLoading) {
    return <p className="text-sm text-muted-foreground">Loading sessions…</p>;
  }
  const list = data?.sessions ?? [];
  if (list.length === 0) {
    return (
      <p className="text-sm text-muted-foreground italic">
        No session yet. Start one above to enter the cockpit.
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
                <Badge variant={s.endedAt ? "outline" : "default"} className="capitalize">
                  {s.endedAt ? "ended" : s.currentPhase}
                </Badge>
              </div>
              <CardDescription>
                {new Date(s.startedAt).toLocaleString()}
                {s.combatRound > 0 ? ` · round ${s.combatRound}` : ""}
              </CardDescription>
            </CardHeader>
          </Card>
        </Link>
      ))}
    </div>
  );
}
