"use client";

import { useState } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { ArrowDown, ArrowUp, Dice6, SkipForward, User, Skull } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { cn } from "@/lib/utils";
import type { ParticipantView, SessionView } from "@/lib/stores/session-store";

interface Props {
  session: SessionView;
  participants: ParticipantView[];
}

export function InitiativeTracker({ session, participants }: Props) {
  const [busy, setBusy] = useState(false);
  const queryClient = useQueryClient();

  const byEntityId = new Map(participants.map((p) => [p.state.entityId, p]));

  async function rollAll() {
    setBusy(true);
    try {
      const res = await fetch(`/api/sessions/${session.id}/initiative`, {
        method: "POST",
      });
      if (!res.ok) throw new Error("Roll initiative failed");
      queryClient.invalidateQueries({ queryKey: ["session", session.id] });
      toast.success("Initiative rolled");
    } catch (e) {
      toast.error(e instanceof Error ? e.message : "Roll failed");
    } finally {
      setBusy(false);
    }
  }

  async function nextTurn() {
    setBusy(true);
    try {
      const res = await fetch(`/api/sessions/${session.id}/turn/next`, {
        method: "POST",
      });
      if (!res.ok) throw new Error("Next turn failed");
      queryClient.invalidateQueries({ queryKey: ["session", session.id] });
    } catch (e) {
      toast.error(e instanceof Error ? e.message : "Next turn failed");
    } finally {
      setBusy(false);
    }
  }

  async function reorder(index: number, direction: -1 | 1) {
    const target = index + direction;
    if (target < 0 || target >= session.initiativeOrder.length) return;
    const next = [...session.initiativeOrder];
    [next[index], next[target]] = [next[target], next[index]];
    await persistOrder(next);
  }

  async function persistOrder(order: typeof session.initiativeOrder) {
    setBusy(true);
    try {
      const res = await fetch(`/api/sessions/${session.id}/initiative`, {
        method: "PATCH",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({ initiativeOrder: order }),
      });
      if (!res.ok) throw new Error("Reorder failed");
      queryClient.invalidateQueries({ queryKey: ["session", session.id] });
    } catch (e) {
      toast.error(e instanceof Error ? e.message : "Reorder failed");
    } finally {
      setBusy(false);
    }
  }

  return (
    <Card className="h-full">
      <CardContent className="space-y-3 pt-4">
        <div className="flex items-center justify-between gap-2">
          <div>
            <h3 className="text-sm font-semibold uppercase tracking-wide">Initiative</h3>
            {session.combatRound > 0 ? (
              <p className="text-xs text-muted-foreground">Round {session.combatRound}</p>
            ) : null}
          </div>
          <div className="flex gap-1">
            <Button size="sm" variant="outline" onClick={rollAll} disabled={busy}>
              <Dice6 className="size-4" /> Roll
            </Button>
            <Button
              size="sm"
              onClick={nextTurn}
              disabled={busy || session.initiativeOrder.length === 0}
            >
              <SkipForward className="size-4" /> Next
            </Button>
          </div>
        </div>

        {session.initiativeOrder.length === 0 ? (
          <p className="text-xs text-muted-foreground italic">
            No participants — add some when starting the session.
          </p>
        ) : (
          <div className="space-y-1">
            {session.initiativeOrder.map((entry, index) => {
              const p = byEntityId.get(entry.entityId);
              const ent = p?.entity;
              const cs = p?.state.currentState;
              const dead = cs?.hp !== undefined && cs.hp <= 0;
              const active = index === session.activeTurnIndex && session.combatRound > 0;
              return (
                <div
                  key={entry.entityId}
                  data-testid="initiative-entry"
                  data-active={active ? "true" : "false"}
                  className={cn(
                    "flex items-center gap-2 rounded border px-2 py-1.5 text-sm",
                    active && "border-primary bg-primary/10",
                    dead && "opacity-50 line-through",
                  )}
                >
                  <Badge variant="secondary" className="font-mono text-xs">
                    {entry.initiative}
                  </Badge>
                  <div className="flex items-center gap-1 flex-1 truncate">
                    {entry.isPlayer ? (
                      <User className="size-3 text-blue-400" />
                    ) : dead ? (
                      <Skull className="size-3 text-muted-foreground" />
                    ) : (
                      <span className="size-3" />
                    )}
                    <span className="truncate">{ent?.name ?? entry.entityId}</span>
                  </div>
                  <div className="flex">
                    <Button
                      size="icon"
                      variant="ghost"
                      className="size-6"
                      onClick={() => reorder(index, -1)}
                      disabled={busy || index === 0}
                      aria-label="Move up"
                    >
                      <ArrowUp className="size-3" />
                    </Button>
                    <Button
                      size="icon"
                      variant="ghost"
                      className="size-6"
                      onClick={() => reorder(index, 1)}
                      disabled={busy || index === session.initiativeOrder.length - 1}
                      aria-label="Move down"
                    >
                      <ArrowDown className="size-3" />
                    </Button>
                  </div>
                </div>
              );
            })}
          </div>
        )}
      </CardContent>
    </Card>
  );
}
