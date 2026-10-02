"use client";

import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { ImageIcon, X, Wind } from "lucide-react";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Card, CardContent } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import type { AudioAsset, EntityRow } from "@/lib/db/schema";
import { useAudioStore } from "@/lib/stores/audio-store";
import { ENTITY_TYPE_LABELS } from "@/lib/engine/catalog";

const PREVIEW_TYPES = ["npc", "monster", "location", "item"];

export function EntityPreview({ campaignId, sessionId }: { campaignId: string; sessionId: string }) {
  const [revealedId, setRevealedId] = useState<string | null>(null);
  const [fullscreen, setFullscreen] = useState(false);

  const { data } = useQuery({
    queryKey: ["entities-with-images", campaignId],
    queryFn: async () => {
      const res = await fetch(`/api/campaigns/${campaignId}/entities`);
      if (!res.ok) throw new Error("Impossible de charger les fiches");
      return res.json() as Promise<{ entities: EntityRow[] }>;
    },
    refetchInterval: 30_000,
  });

  const candidates = (data?.entities ?? []).filter(
    (e) => PREVIEW_TYPES.includes(e.type) && e.imageUrl,
  );
  const revealed = candidates.find((e) => e.id === revealedId);

  const playAmbience = useAudioStore((s) => s.playAmbience);

  const { data: audioData } = useQuery({
    queryKey: ["audio-assets"],
    queryFn: async () => {
      const res = await fetch("/api/audio/assets");
      if (!res.ok) throw new Error("Impossible de charger l’audio");
      return res.json() as Promise<{ assets: AudioAsset[] }>;
    },
  });

  function findAmbienceFor(entity: EntityRow): AudioAsset | null {
    const all = audioData?.assets ?? [];
    const ambiences = all.filter((a) => a.type === "ambience");
    if (entity.type !== "location") return null;
    const attrs = (entity.attributes ?? {}) as Record<string, unknown>;
    const explicitId = typeof attrs.defaultAmbienceId === "string" ? attrs.defaultAmbienceId : null;
    if (explicitId) {
      const byId = ambiences.find((a) => a.id === explicitId || a.name === explicitId);
      if (byId) return byId;
    }
    const explicitName = typeof attrs.defaultAmbienceName === "string" ? attrs.defaultAmbienceName : null;
    if (explicitName) {
      const byName = ambiences.find((a) => a.name.toLowerCase() === explicitName.toLowerCase());
      if (byName) return byName;
    }
    // Fallback: match by tag overlap
    const entityTags = new Set([...(entity.tags ?? []), entity.name.toLowerCase()]);
    return (
      ambiences.find((a) =>
        a.tags.some((t) => Array.from(entityTags).some((et) => et.toLowerCase().includes(t.toLowerCase()))),
      ) ?? null
    );
  }

  const revealedAmbience = revealed ? findAmbienceFor(revealed) : null;

  return (
    <Card>
      <CardContent className="pt-4 space-y-3">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            <ImageIcon className="size-4 text-muted-foreground" />
            <h3 className="text-sm font-semibold uppercase tracking-wide">
              Révéler
            </h3>
          </div>
          {candidates.length > 0 ? (
            <Select
              value={revealedId ?? ""}
              onValueChange={(v) => setRevealedId(v || null)}
            >
              <SelectTrigger className="w-56">
                <SelectValue placeholder="Choisir une fiche à montrer…" />
              </SelectTrigger>
              <SelectContent>
                {candidates.map((e) => (
                  <SelectItem key={e.id} value={e.id}>
                    {e.name} <span className="text-muted-foreground ml-1">({ENTITY_TYPE_LABELS[e.type]})</span>
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          ) : null}
        </div>

        {candidates.length === 0 ? (
          <p className="text-xs text-muted-foreground italic">
            Aucune fiche n’a encore d’image. Générez-en une depuis l’éditeur de fiche.
          </p>
        ) : !revealed ? (
          <p className="text-xs text-muted-foreground italic">
            Choisissez un PNJ, un monstre, un lieu ou un objet pour afficher son image.
          </p>
        ) : (
          <div className="space-y-2">
            <div className="flex items-center justify-between">
              <div>
                <h4 className="font-semibold">{revealed.name}</h4>
                <Badge variant="outline" className="text-xs mt-1">
                  {ENTITY_TYPE_LABELS[revealed.type]}
                </Badge>
              </div>
              <div className="flex gap-1">
                {revealedAmbience ? (
                  <Button
                    size="sm"
                    variant="outline"
                    onClick={() => {
                      playAmbience(revealedAmbience);
                      toast.success(`Ambiance : ${revealedAmbience.name}`);
                    }}
                  >
                    <Wind className="size-4" /> Ambiance
                  </Button>
                ) : null}
                <Button
                  size="sm"
                  onClick={async () => {
                    const res = await fetch(`/api/sessions/${sessionId}/map`, {
                      method: "POST",
                      headers: { "content-type": "application/json" },
                      body: JSON.stringify({ op: "spotlight", entityId: revealed.id }),
                    });
                    if (res.ok) toast.success(`${revealed.name} est montré aux joueurs`);
                    else toast.error("Impossible de montrer aux joueurs");
                  }}
                >
                  Montrer aux joueurs
                </Button>
                <Button size="sm" variant="outline" onClick={() => setFullscreen(true)}>
                  Plein écran
                </Button>
                <Button size="icon" variant="ghost" onClick={() => setRevealedId(null)} aria-label="Fermer">
                  <X className="size-4" />
                </Button>
              </div>
            </div>
            {/* eslint-disable-next-line @next/next/no-img-element */}
            <img
              src={revealed.imageUrl ?? ""}
              alt={revealed.name}
              className="rounded border max-h-72 w-full object-cover bg-black/30"
            />
            {revealed.description ? (
              <p className="text-xs text-muted-foreground italic">
                {revealed.description}
              </p>
            ) : null}
          </div>
        )}
      </CardContent>

      <Dialog open={fullscreen} onOpenChange={setFullscreen}>
        <DialogContent className="max-w-5xl">
          <DialogHeader>
            <DialogTitle>{revealed?.name}</DialogTitle>
          </DialogHeader>
          {revealed?.imageUrl ? (
            // eslint-disable-next-line @next/next/no-img-element
            <img
              src={revealed.imageUrl}
              alt={revealed.name}
              className="rounded border w-full max-h-[80vh] object-contain bg-black/40"
            />
          ) : null}
          {revealed?.description ? (
            <p className="text-sm text-muted-foreground italic">{revealed.description}</p>
          ) : null}
        </DialogContent>
      </Dialog>
    </Card>
  );
}
