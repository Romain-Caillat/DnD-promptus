"use client";

import { useQuery } from "@tanstack/react-query";
import { Badge } from "@/components/ui/badge";
import { Card, CardContent } from "@/components/ui/card";
import { ScrollArea } from "@/components/ui/scroll-area";
import type { ResolutionRecord } from "@/lib/engine/types";
import { OUTCOME_LABELS } from "@/lib/engine/catalog";
import type { SessionTimelineRow } from "@/lib/db/schema";

export function TimelineFeed({ sessionId }: { sessionId: string }) {
  const { data, isLoading } = useQuery({
    queryKey: ["timeline", sessionId],
    queryFn: async () => {
      const res = await fetch(`/api/sessions/${sessionId}/timeline`);
      if (!res.ok) throw new Error("Impossible de charger le journal");
      return res.json() as Promise<{ timeline: SessionTimelineRow[] }>;
    },
    refetchInterval: 3000,
  });

  return (
    <Card className="h-full">
      <CardContent className="pt-4 space-y-2">
        <h3 className="text-sm font-semibold uppercase tracking-wide">Journal</h3>
        {isLoading ? (
          <p className="text-xs text-muted-foreground">Chargement…</p>
        ) : (data?.timeline ?? []).length === 0 ? (
          <p className="text-xs text-muted-foreground italic">
            Aucune action résolue pour l’instant.
          </p>
        ) : (
          <ScrollArea className="h-72">
            <ul className="space-y-1 pr-2">
              {data!.timeline.map((row) => {
                const r = row.resolutionRecord as ResolutionRecord | null;
                const outcome = r?.outcome ?? "none";
                return (
                  <li
                    key={row.id}
                    className="text-xs leading-tight border-l-2 pl-2 py-1"
                    data-testid="timeline-entry"
                  >
                    <div className="flex items-center gap-2">
                      <Badge
                        variant={
                          outcome === "success"
                            ? "default"
                            : outcome === "fail"
                            ? "destructive"
                            : "outline"
                        }
                        className="text-[10px] py-0"
                      >
                        {OUTCOME_LABELS[outcome]}
                      </Badge>
                      {row.round !== null ? (
                        <span className="text-[10px] text-muted-foreground">
                          R{row.round}
                        </span>
                      ) : null}
                    </div>
                    <p className="mt-0.5">{row.description}</p>
                  </li>
                );
              })}
            </ul>
          </ScrollArea>
        )}
      </CardContent>
    </Card>
  );
}
