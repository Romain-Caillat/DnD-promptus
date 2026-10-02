"use client";

import { useState } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { Hand } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { useRuleset } from "@/components/providers/ruleset-provider";
import { skillLabel } from "@/lib/engine/ruleset";

interface RequestRow {
  id: string;
  actionId: string;
  attackId: string | null;
  targetIds: string[];
  label: string;
  note: string | null;
  status: "pending" | "resolved" | "rejected";
  result: string | null;
  playerName: string;
  characterName: string | null;
}

export function RequestsPanel({ sessionId }: { sessionId: string }) {
  const ruleset = useRuleset();
  const queryClient = useQueryClient();
  const [dcs, setDcs] = useState<Record<string, string>>({});
  const [busy, setBusy] = useState<string | null>(null);
  const { data } = useQuery({
    queryKey: ["player-requests", sessionId],
    queryFn: async () => {
      const res = await fetch(`/api/sessions/${sessionId}/requests`);
      if (!res.ok) throw new Error("Impossible de charger les demandes");
      return (await res.json()) as { requests: RequestRow[] };
    },
  });
  const pending = (data?.requests ?? []).filter((r) => r.status === "pending");
  const done = (data?.requests ?? []).filter((r) => r.status !== "pending").slice(0, 4);
  if (!pending.length && !done.length) return null;

  async function decide(id: string, body: object) {
    setBusy(id);
    try {
      const res = await fetch(`/api/sessions/${sessionId}/requests/${id}`, {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify(body),
      });
      const json = await res.json();
      if (!res.ok) throw new Error(json.error?.message ?? "Échec");
      if (json.result) toast.info(json.result);
      void queryClient.invalidateQueries({ queryKey: ["player-requests", sessionId] });
      void queryClient.invalidateQueries({ queryKey: ["timeline", sessionId] });
      void queryClient.invalidateQueries({ queryKey: ["session", sessionId] });
    } catch (e) {
      toast.error(e instanceof Error ? e.message : "Échec");
    } finally {
      setBusy(null);
    }
  }

  return (
    <Card data-testid="requests-panel" className={pending.length ? "border-amber-500" : undefined}>
      <CardContent className="pt-4 space-y-3">
        <h3 className="text-sm font-semibold uppercase tracking-wide flex items-center gap-1">
          <Hand className="size-4" /> Demandes des joueurs {pending.length ? `(${pending.length})` : ""}
        </h3>
        {pending.map((r) => {
          const action = ruleset.actions.find((a) => a.id === r.actionId);
          const testable = !!(action?.skill || action?.ability);
          return (
            <div key={r.id} className="rounded border p-2 space-y-2 text-sm" data-testid="pending-request">
              <p>
                <span className="font-medium">{r.characterName ?? r.playerName}</span> : {r.label}
                {action?.skill ? <span className="text-muted-foreground"> ({skillLabel(ruleset, action.skill)})</span> : null}
              </p>
              {r.note ? <p className="text-xs italic text-muted-foreground">« {r.note} »</p> : null}
              <div className="flex flex-wrap items-center gap-1">
                {r.attackId ? (
                  <>
                    <Button size="xs" disabled={busy === r.id} onClick={() => decide(r.id, { decision: "attack" })}>
                      Résoudre l’attaque
                    </Button>
                    <Button
                      size="xs"
                      variant="ghost"
                      title="Ignorer la portée et la ligne de vue"
                      disabled={busy === r.id}
                      onClick={() => decide(r.id, { decision: "attack", force: true })}
                    >
                      Forcer
                    </Button>
                  </>
                ) : testable ? (
                  <>
                    <Input
                      className="h-7 w-16 text-xs"
                      type="number"
                      placeholder="DD"
                      aria-label="Difficulté"
                      value={dcs[r.id] ?? ""}
                      onChange={(e) => setDcs((d) => ({ ...d, [r.id]: e.target.value }))}
                    />
                    <Button
                      size="xs"
                      disabled={busy === r.id || !dcs[r.id]}
                      onClick={() => decide(r.id, { decision: "roll", dc: Number(dcs[r.id]) })}
                    >
                      Lancer le test
                    </Button>
                  </>
                ) : null}
                <Button size="xs" variant="outline" disabled={busy === r.id} onClick={() => decide(r.id, { decision: "accept" })}>
                  Valider
                </Button>
                <Button size="xs" variant="ghost" disabled={busy === r.id} onClick={() => decide(r.id, { decision: "reject" })}>
                  Refuser
                </Button>
              </div>
            </div>
          );
        })}
        {done.length ? (
          <ul className="text-xs text-muted-foreground space-y-0.5">
            {done.map((r) => (
              <li key={r.id}>
                {r.characterName ?? r.playerName} · {r.label} : {r.result}
              </li>
            ))}
          </ul>
        ) : null}
      </CardContent>
    </Card>
  );
}
