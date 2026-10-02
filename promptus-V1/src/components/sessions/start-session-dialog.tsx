"use client";

import { useState } from "react";
import { useRouter } from "next/navigation";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import { ENTITY_TYPE_LABELS } from "@/lib/engine/catalog";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Checkbox } from "@/components/ui/checkbox";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Badge } from "@/components/ui/badge";
import type { EntityRow } from "@/lib/db/schema";

const PARTICIPANT_TYPES = ["character", "monster", "npc"] as const;

export function StartSessionDialog({ campaignId }: { campaignId: string }) {
  const [open, setOpen] = useState(false);
  const [name, setName] = useState("");
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [busy, setBusy] = useState(false);
  const router = useRouter();
  const queryClient = useQueryClient();

  const { data, isLoading } = useQuery({
    queryKey: ["entities-participants", campaignId, open],
    enabled: open,
    queryFn: async () => {
      const res = await fetch(`/api/campaigns/${campaignId}/entities`);
      if (!res.ok) throw new Error("Impossible de charger les fiches");
      return res.json() as Promise<{ entities: EntityRow[] }>;
    },
  });

  const candidates = (data?.entities ?? []).filter(
    (e) => !e.attributes.copyOf && PARTICIPANT_TYPES.includes(e.type as (typeof PARTICIPANT_TYPES)[number]),
  );

  function toggle(id: string) {
    setSelected((s) => {
      const next = new Set(s);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  }

  async function start() {
    if (!name.trim()) {
      toast.error("Le nom de la session est obligatoire");
      return;
    }
    if (selected.size === 0) {
      toast.error("Choisissez au moins un participant");
      return;
    }
    setBusy(true);
    try {
      const res = await fetch(`/api/campaigns/${campaignId}/sessions`, {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({
          name: name.trim(),
          participantEntityIds: Array.from(selected),
        }),
      });
      if (!res.ok) {
        const err = await res.json();
        throw new Error(err.error?.message ?? "Impossible de lancer la session");
      }
      const { session } = await res.json();
      toast.success(`Session « ${session.name} » lancée`);
      setOpen(false);
      setName("");
      setSelected(new Set());
      queryClient.invalidateQueries({ queryKey: ["sessions", campaignId] });
      router.push(`/sessions/${session.id}`);
    } catch (e) {
      toast.error(e instanceof Error ? e.message : "Erreur inconnue");
    } finally {
      setBusy(false);
    }
  }

  return (
    <Dialog open={open} onOpenChange={setOpen}>
      <DialogTrigger asChild>
        <Button>Lancer une session</Button>
      </DialogTrigger>
      <DialogContent className="max-w-lg">
        <DialogHeader>
          <DialogTitle>Lancer une nouvelle session</DialogTitle>
          <DialogDescription>
            Pick a name and the participants (PCs, monsters and NPCs that will be present).
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-4">
          <div className="space-y-1">
            <Label htmlFor="session-name">Nom de la session</Label>
            <Input
              id="session-name"
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="ex. La Crypte du Roi oublié"
              autoFocus
            />
          </div>
          <div>
            <Label className="text-xs uppercase tracking-wide text-muted-foreground">
              Participants
            </Label>
            {isLoading ? (
              <p className="text-sm text-muted-foreground py-4">Chargement des fiches…</p>
            ) : candidates.length === 0 ? (
              <p className="text-sm text-muted-foreground py-4">
                Aucun personnage, monstre ou PNJ dans cette campagne.
              </p>
            ) : (
              <ScrollArea className="h-64 rounded border p-2">
                <div className="space-y-1">
                  {candidates.map((c) => (
                    <label
                      key={c.id}
                      className="flex items-center gap-3 px-2 py-2 rounded hover:bg-muted cursor-pointer"
                    >
                      <Checkbox
                        checked={selected.has(c.id)}
                        onCheckedChange={() => toggle(c.id)}
                      />
                      <span className="flex-1">{c.name}</span>
                      <Badge variant="outline">
                        {ENTITY_TYPE_LABELS[c.type]}
                      </Badge>
                    </label>
                  ))}
                </div>
              </ScrollArea>
            )}
          </div>
        </div>

        <DialogFooter>
          <Button onClick={start} disabled={busy}>
            {busy ? "Lancement…" : `Lancer (${selected.size})`}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
