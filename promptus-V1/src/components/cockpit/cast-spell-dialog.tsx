"use client";

import { useState } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Checkbox } from "@/components/ui/checkbox";
import { Badge } from "@/components/ui/badge";
import { ScrollArea } from "@/components/ui/scroll-area";
import type { ParticipantView } from "@/lib/stores/session-store";
import type { ResolutionRecord } from "@/lib/engine/types";
import type { EntityRow } from "@/lib/db/schema";

interface Props {
  sessionId: string;
  campaignId: string;
  participants: ParticipantView[];
  trigger: React.ReactNode;
}

export function CastSpellDialog({ sessionId, campaignId, participants, trigger }: Props) {
  const [open, setOpen] = useState(false);
  const [casterId, setCasterId] = useState<string>("");
  const [spellId, setSpellId] = useState<string>("");
  const [targetIds, setTargetIds] = useState<Set<string>>(new Set());
  const [busy, setBusy] = useState(false);
  const [results, setResults] = useState<ResolutionRecord[] | null>(null);
  const queryClient = useQueryClient();

  const { data: spellsData } = useQuery({
    queryKey: ["entities-spells", campaignId, open],
    enabled: open,
    queryFn: async () => {
      const res = await fetch(`/api/campaigns/${campaignId}/entities?type=spell`);
      if (!res.ok) throw new Error("Failed to load spells");
      return res.json() as Promise<{ entities: EntityRow[] }>;
    },
  });

  function handleOpenChange(next: boolean) {
    setOpen(next);
    if (!next) {
      setResults(null);
      setTargetIds(new Set());
      setSpellId("");
    }
  }

  const aliveParticipants = participants.filter(
    (p) => (p.state.currentState.hp ?? 1) > 0,
  );
  const spells = spellsData?.entities ?? [];
  const selectedSpell = spells.find((s) => s.id === spellId);

  function toggleTarget(id: string) {
    setTargetIds((s) => {
      const next = new Set(s);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  }

  async function cast() {
    if (!casterId) return toast.error("Pick a caster");
    if (!spellId) return toast.error("Pick a spell");
    setBusy(true);
    try {
      const res = await fetch(`/api/sessions/${sessionId}/actions`, {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({
          kind: "cast_spell",
          casterId,
          spellEntityId: spellId,
          targetIds: Array.from(targetIds),
        }),
      });
      if (!res.ok) {
        const err = await res.json();
        throw new Error(err.error?.message ?? "Spell cast failed");
      }
      const { records } = (await res.json()) as { records: ResolutionRecord[] };
      setResults(records);
      queryClient.invalidateQueries({ queryKey: ["session", sessionId] });
      queryClient.invalidateQueries({ queryKey: ["timeline", sessionId] });
    } catch (e) {
      toast.error(e instanceof Error ? e.message : "Spell cast failed");
    } finally {
      setBusy(false);
    }
  }

  return (
    <Dialog open={open} onOpenChange={handleOpenChange}>
      <DialogTrigger asChild>{trigger}</DialogTrigger>
      <DialogContent className="max-w-xl">
        <DialogHeader>
          <DialogTitle>Cast a spell</DialogTitle>
        </DialogHeader>

        {results ? (
          <div className="space-y-2" data-testid="cast-results">
            <h3 className="text-sm font-semibold">Resolution</h3>
            <ScrollArea className="max-h-80 rounded border p-3">
              <ul className="space-y-1 text-sm">
                {results.map((r, i) => (
                  <li key={i} className="border-l-2 pl-2 py-0.5">
                    <Badge
                      variant={
                        r.outcome === "success"
                          ? "default"
                          : r.outcome === "fail"
                          ? "destructive"
                          : "outline"
                      }
                      className="mr-2 capitalize"
                    >
                      {r.outcome}
                    </Badge>
                    {r.description}
                  </li>
                ))}
              </ul>
            </ScrollArea>
            <DialogFooter>
              <Button variant="outline" onClick={() => setResults(null)}>
                Cast another
              </Button>
              <Button onClick={() => handleOpenChange(false)}>Close</Button>
            </DialogFooter>
          </div>
        ) : (
          <div className="space-y-3">
            <div className="space-y-1">
              <Label className="text-xs uppercase">Caster</Label>
              <Select value={casterId} onValueChange={setCasterId}>
                <SelectTrigger>
                  <SelectValue placeholder="Pick a caster…" />
                </SelectTrigger>
                <SelectContent>
                  {aliveParticipants.map((p) => (
                    <SelectItem key={p.state.entityId} value={p.state.entityId}>
                      {p.entity?.name ?? p.state.entityId}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>

            <div className="space-y-1">
              <Label className="text-xs uppercase">Spell</Label>
              <Select value={spellId} onValueChange={setSpellId}>
                <SelectTrigger>
                  <SelectValue placeholder={spells.length === 0 ? "No spells in this campaign" : "Pick a spell…"} />
                </SelectTrigger>
                <SelectContent>
                  {spells.map((s) => (
                    <SelectItem key={s.id} value={s.id}>
                      {s.name}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
              {selectedSpell?.description ? (
                <p className="text-xs text-muted-foreground italic mt-1">
                  {selectedSpell.description}
                </p>
              ) : null}
            </div>

            <div>
              <Label className="text-xs uppercase">Target(s)</Label>
              <div className="rounded border p-2 max-h-40 overflow-auto space-y-1">
                {aliveParticipants
                  .filter((p) => p.state.entityId !== casterId)
                  .map((p) => (
                    <label
                      key={p.state.entityId}
                      className="flex items-center gap-2 px-1 py-1 rounded hover:bg-muted cursor-pointer"
                    >
                      <Checkbox
                        checked={targetIds.has(p.state.entityId)}
                        onCheckedChange={() => toggleTarget(p.state.entityId)}
                      />
                      <span className="flex-1 text-sm">{p.entity?.name}</span>
                    </label>
                  ))}
              </div>
            </div>

            <DialogFooter>
              <Button onClick={cast} disabled={busy} data-testid="cast-submit">
                {busy ? "Casting…" : "Resolve spell"}
              </Button>
            </DialogFooter>
          </div>
        )}
      </DialogContent>
    </Dialog>
  );
}
