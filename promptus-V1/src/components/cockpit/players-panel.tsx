"use client";

import { useQuery } from "@tanstack/react-query";
import { toast } from "sonner";
import { Copy, Users } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import type { RealtimeState } from "@/lib/realtime/use-realtime";
import { cn } from "@/lib/utils";

interface PlayersData {
  inviteCode: string;
  players: { id: string; name: string; characterName: string | null }[];
}

export function PlayersPanel({ sessionId, realtime }: { sessionId: string; realtime: RealtimeState }) {
  const { data } = useQuery({
    queryKey: ["session-players", sessionId],
    queryFn: async () => {
      const res = await fetch(`/api/sessions/${sessionId}/players`);
      if (!res.ok) throw new Error("Impossible de charger les joueurs");
      return (await res.json()) as PlayersData;
    },
  });
  const online = new Set(realtime.players.map((p) => p.playerId));
  const link = data && typeof window !== "undefined" ? `${window.location.origin}/play/${data.inviteCode}` : "";

  return (
    <Card data-testid="players-panel">
      <CardContent className="pt-4 space-y-3">
        <div className="flex items-center justify-between">
          <h3 className="text-sm font-semibold uppercase tracking-wide flex items-center gap-1">
            <Users className="size-4" /> Joueurs
          </h3>
          <span
            className={cn("text-xs flex items-center gap-1", realtime.connected ? "text-emerald-500" : "text-muted-foreground")}
            data-testid="realtime-status"
          >
            <span className={cn("size-2 rounded-full", realtime.connected ? "bg-emerald-500" : "bg-muted-foreground")} />
            {realtime.connected ? "Temps réel" : "Hors ligne"}
          </span>
        </div>
        {data ? (
          <div className="flex gap-1">
            <input readOnly value={link} className="flex-1 min-w-0 rounded border bg-muted/40 px-2 py-1 text-xs font-mono" aria-label="Lien d’invitation" />
            <Button
              size="icon-sm"
              variant="outline"
              aria-label="Copier le lien"
              onClick={() => {
                void navigator.clipboard?.writeText(link);
                toast.success("Lien copié");
              }}
            >
              <Copy />
            </Button>
          </div>
        ) : null}
        {data?.players.length ? (
          <ul className="space-y-1 text-sm">
            {data.players.map((p) => (
              <li key={p.id} className="flex items-center gap-2" data-testid="player-row">
                <span className={cn("size-2 rounded-full", online.has(p.id) ? "bg-emerald-500" : "bg-muted")} />
                <span>{p.name}</span>
                <span className="text-xs text-muted-foreground">{p.characterName ?? "spectateur"}</span>
              </li>
            ))}
          </ul>
        ) : (
          <p className="text-xs text-muted-foreground italic">Envoyez le lien : les joueurs choisissent leur personnage.</p>
        )}
      </CardContent>
    </Card>
  );
}
