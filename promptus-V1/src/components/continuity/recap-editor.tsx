"use client";

import { useState } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { Copy, Eye, EyeOff, Loader2, Save, Sparkles } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { Label } from "@/components/ui/label";
import { Textarea } from "@/components/ui/textarea";
import type { SessionRecap } from "@/lib/continuity/recap";

/** Récapitulatifs de fin de session : relus, corrigés et publiés par le MJ. */
export function RecapEditor({ sessionId, recap }: { sessionId: string; recap: SessionRecap }) {
  const queryClient = useQueryClient();
  // Les textes suivent le serveur tant que le MJ ne les a pas modifiés.
  const [draft, setDraft] = useState<{ base: string; players: string; gm: string } | null>(null);
  const base = recap.generatedAt;
  const players = draft?.base === base ? draft.players : recap.players;
  const gm = draft?.base === base ? draft.gm : recap.gm;
  const dirty = players !== recap.players || gm !== recap.gm;
  const [busy, setBusy] = useState(false);
  const running = recap.llm?.status === "running";

  async function call(method: "PUT" | "POST", body?: object, ok?: string) {
    setBusy(true);
    try {
      const res = await fetch(`/api/sessions/${sessionId}/recap`, {
        method,
        headers: { "content-type": "application/json" },
        body: body ? JSON.stringify(body) : undefined,
      });
      const json = await res.json();
      if (!res.ok) throw new Error(json.error?.message ?? "Action impossible");
      setDraft(null);
      void queryClient.invalidateQueries({ queryKey: ["session", sessionId] });
      if (ok) toast.success(ok);
    } catch (e) {
      toast.error(e instanceof Error ? e.message : "Action impossible");
    } finally {
      setBusy(false);
    }
  }

  const f = recap.facts;
  return (
    <Card className="border-primary/60" data-testid="recap-editor">
      <CardContent className="pt-4 space-y-4">
        <div className="flex flex-wrap items-center justify-between gap-2">
          <h2 className="text-sm font-semibold uppercase tracking-wide">Récapitulatif de la session</h2>
          <div className="flex items-center gap-2 text-xs">
            <Badge variant={recap.status === "published" ? "default" : "outline"} data-testid="recap-status">
              {recap.status === "published" ? "publié aux joueurs" : "brouillon"}
            </Badge>
            <Badge variant="secondary">{recap.source === "llm" ? "rédigé par l’IA" : "factuel"}</Badge>
          </div>
        </div>
        <div className="flex flex-wrap gap-1 text-xs">
          <Badge variant="outline">{f.durationMinutes} min</Badge>
          <Badge variant="outline">{f.scenesVisited.length} scène(s)</Badge>
          <Badge variant="outline">{f.cluesFound.length} indice(s)</Badge>
          <Badge variant="outline">{f.revelationsLearned.length} révélation(s)</Badge>
          <Badge variant="outline">{f.combats} combat(s)</Badge>
          {f.frontsAdvanced.map((x) => (
            <Badge key={x.id} variant="destructive">
              {x.name} : {x.to + 1}/{x.total}
            </Badge>
          ))}
        </div>
        {running ? (
          <p className="text-xs text-muted-foreground flex items-center gap-1" data-testid="recap-running">
            <Loader2 className="size-3 animate-spin" /> L’IA rédige le récapitulatif…
          </p>
        ) : recap.llm?.status === "failed" ? (
          <p className="text-xs text-destructive">L’IA n’a pas pu rédiger : {recap.llm.error}</p>
        ) : null}
        <div className="space-y-1">
          <Label htmlFor="recap-players" className="text-xs uppercase tracking-wide text-muted-foreground">
            « Précédemment… » pour les joueurs (aucun secret)
          </Label>
          <Textarea id="recap-players" rows={6} value={players} onChange={(e) => setDraft({ base, players: e.target.value, gm })} />
        </div>
        <div className="space-y-1">
          <Label htmlFor="recap-gm" className="text-xs uppercase tracking-wide text-muted-foreground">
            Notes MJ : fils ouverts, conséquences, pistes
          </Label>
          <Textarea id="recap-gm" rows={8} value={gm} onChange={(e) => setDraft({ base, players, gm: e.target.value })} />
        </div>
        <div className="flex flex-wrap gap-2">
          <Button size="sm" variant="outline" disabled={busy || !dirty} onClick={() => call("PUT", { players, gm }, "Récapitulatif enregistré")}>
            <Save /> Enregistrer
          </Button>
          {recap.status === "published" ? (
            <Button size="sm" variant="outline" disabled={busy} onClick={() => call("PUT", { players, gm, publish: false }, "Récapitulatif retiré")}>
              <EyeOff /> Retirer aux joueurs
            </Button>
          ) : (
            <Button size="sm" disabled={busy || running} onClick={() => call("PUT", { players, gm, publish: true }, "Publié aux joueurs")}>
              <Eye /> Publier aux joueurs
            </Button>
          )}
          <Button size="sm" variant="outline" disabled={busy || running} onClick={() => call("POST", undefined, "L’IA rédige le récapitulatif")}>
            <Sparkles /> {recap.source === "llm" ? "Réécrire avec l’IA" : "Rédiger avec l’IA"}
          </Button>
          <Button
            size="sm"
            variant="ghost"
            onClick={() => {
              void navigator.clipboard?.writeText(players);
              toast.success("« Précédemment… » copié");
            }}
          >
            <Copy /> Copier
          </Button>
        </div>
      </CardContent>
    </Card>
  );
}
