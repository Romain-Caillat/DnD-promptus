"use client";

import { useState } from "react";
import Link from "next/link";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { ExternalLink, ImageIcon, Loader2, RefreshCw, Sparkles, Video } from "lucide-react";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Textarea } from "@/components/ui/textarea";
import { ENTITY_TYPE_LABELS } from "@/lib/engine/catalog";
import { isVideo, type MediaTarget } from "@/lib/media/prompts";
import type { MediaItem, MediaProviders } from "@/lib/media/server";
import { parseYouTube, youTubeSearchUrl } from "@/lib/media/youtube";

interface Overview {
  items: MediaItem[];
  music: { sceneId: string; title: string; musicUrl: string | null; musicQuery: string | null }[];
  providers: MediaProviders;
  unitCostUsd: { image: number; video: number };
  budgetUsd: number | null;
  spentUsd: number;
}

interface Pending {
  title: string;
  items: { target: MediaTarget; prompt?: string }[];
  regenerate: boolean;
  estimate: number;
}

const usd = (n: number) => `${n.toLocaleString("fr-FR", { minimumFractionDigits: 2, maximumFractionDigits: 2 })} $`;

export function MediaStudio({ campaignId }: { campaignId: string }) {
  const queryClient = useQueryClient();
  const key = ["media", campaignId];
  const { data } = useQuery({
    queryKey: key,
    queryFn: async () => {
      const res = await fetch(`/api/campaigns/${campaignId}/media`);
      if (!res.ok) throw new Error("Impossible de charger les médias");
      return (await res.json()) as Overview;
    },
    // Suivi des générations en cours.
    refetchInterval: (q) => (q.state.data?.items.some((i) => i.status === "running") ? 2000 : false),
  });
  const [prompts, setPrompts] = useState<Record<string, string>>({});
  const [pending, setPending] = useState<Pending | null>(null);
  const [music, setMusic] = useState<Record<string, string>>({});

  if (!data) return <p className="text-muted-foreground">Chargement…</p>;
  const cost = (t: MediaTarget) => (isVideo(t) ? data.unitCostUsd.video : data.unitCostUsd.image);
  const available = (t: MediaTarget) => (isVideo(t) ? data.providers.video.available : data.providers.image.available);
  const remaining = data.budgetUsd === null ? null : Math.max(0, data.budgetUsd - data.spentUsd);

  function ask(title: string, list: MediaItem[], regenerate: boolean) {
    const items = list.map((i) => ({ target: i.target, prompt: prompts[i.key] }));
    setPending({ title, items, regenerate, estimate: list.reduce((s, i) => s + cost(i.target), 0) });
  }

  async function launch(p: Pending) {
    setPending(null);
    const res = await fetch(`/api/campaigns/${campaignId}/media`, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ items: p.items, regenerate: p.regenerate }),
    });
    const json = await res.json();
    if (!res.ok) return toast.error(json.error?.message ?? "Génération impossible");
    toast.success(json.queued ? `${json.queued} média(s) en cours de génération` : "Rien à générer");
    void queryClient.invalidateQueries({ queryKey: key });
  }

  async function saveMusic(sceneId: string, url: string) {
    const res = await fetch(`/api/campaigns/${campaignId}/media`, {
      method: "PATCH",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ sceneId, musicUrl: url }),
    });
    const json = await res.json();
    if (!res.ok) return toast.error(json.error?.message ?? "Enregistrement impossible");
    toast.success(url ? "Musique enregistrée" : "Musique retirée");
    void queryClient.invalidateQueries({ queryKey: key });
  }

  const missingImages = data.items.filter((i) => !isVideo(i.target) && i.status === "none");
  const missingVideos = data.items.filter((i) => isVideo(i.target) && i.status === "none");
  const scenes = [...new Set(data.items.filter((i) => i.group === "scenes").map((i) => i.target.id))];

  function itemCard(i: MediaItem, compact = false) {
    const running = i.status === "running";
    return (
      <div key={i.key} className="space-y-2" data-testid={`media-${i.key}`} data-status={i.status}>
        <div className="aspect-video w-full overflow-hidden rounded border bg-muted/40 flex items-center justify-center">
          {running ? (
            <Loader2 className="size-6 animate-spin text-muted-foreground" />
          ) : i.url ? (
            isVideo(i.target) ? (
              <video src={i.url} controls playsInline className="h-full w-full object-cover" />
            ) : (
              // eslint-disable-next-line @next/next/no-img-element
              <img src={i.url} alt={i.label} className="h-full w-full object-cover" />
            )
          ) : isVideo(i.target) ? (
            <Video className="size-6 text-muted-foreground" />
          ) : (
            <ImageIcon className="size-6 text-muted-foreground" />
          )}
        </div>
        {!compact ? (
          <div className="flex items-center justify-between gap-2">
            <div className="min-w-0">
              <p className="text-sm font-medium truncate">{i.label}</p>
              <p className="text-xs text-muted-foreground">{ENTITY_TYPE_LABELS[i.detail as keyof typeof ENTITY_TYPE_LABELS] ?? i.detail}</p>
            </div>
          </div>
        ) : (
          <p className="text-xs text-muted-foreground">{i.detail}</p>
        )}
        {i.status === "failed" ? <p className="text-xs text-destructive">{i.error}</p> : null}
        <details className="text-xs">
          <summary className="cursor-pointer text-muted-foreground">Prompt</summary>
          <Textarea
            className="mt-1 text-xs"
            rows={4}
            aria-label={`Prompt ${i.label} ${i.detail}`}
            value={prompts[i.key] ?? i.prompt}
            onChange={(e) => setPrompts((p) => ({ ...p, [i.key]: e.target.value }))}
          />
        </details>
        <Button
          size="xs"
          variant={i.url ? "outline" : "default"}
          disabled={running || !available(i.target)}
          onClick={() => ask(`${i.url ? "Régénérer" : "Générer"} : ${i.label}`, [i], !!i.url)}
        >
          {i.url ? <RefreshCw /> : <Sparkles />} {i.url ? "Régénérer" : "Générer"} (≈ {usd(cost(i.target))})
        </Button>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      <Card>
        <CardContent className="pt-4 grid gap-4 md:grid-cols-3 text-sm">
          <div>
            <p className="text-xs uppercase tracking-wide text-muted-foreground">Images</p>
            <p data-testid="image-provider">
              {data.providers.image.available ? `${data.providers.image.model} (${data.providers.image.provider})` : "Non configuré : ajoutez OPENROUTER_API_KEY"}
            </p>
            <p className="text-xs text-muted-foreground">≈ {usd(data.unitCostUsd.image)} par image</p>
          </div>
          <div>
            <p className="text-xs uppercase tracking-wide text-muted-foreground">Vidéos</p>
            <p>{data.providers.video.available ? data.providers.video.model : "Aucun modèle vidéo choisi"}</p>
            <p className="text-xs text-muted-foreground">
              ≈ {usd(data.unitCostUsd.video)} par vidéo ·{" "}
              <Link href={`/campaigns/${campaignId}/settings`} className="underline">
                modèles
              </Link>
            </p>
          </div>
          <div>
            <p className="text-xs uppercase tracking-wide text-muted-foreground">Budget IA</p>
            <p data-testid="media-budget">
              {usd(data.spentUsd)} dépensés{data.budgetUsd !== null ? ` sur ${usd(data.budgetUsd)}` : " (sans limite)"}
            </p>
            {remaining !== null ? <p className="text-xs text-muted-foreground">Reste {usd(remaining)}</p> : null}
          </div>
        </CardContent>
      </Card>

      <div className="flex flex-wrap gap-2">
        <Button
          disabled={!missingImages.length || !data.providers.image.available}
          onClick={() => ask(`Générer ${missingImages.length} image(s) manquante(s)`, missingImages, false)}
        >
          <ImageIcon /> Générer les images manquantes ({missingImages.length})
        </Button>
        <Button
          variant="outline"
          disabled={!missingVideos.length || !data.providers.video.available}
          onClick={() => ask(`Générer ${missingVideos.length} vidéo(s) d’intro`, missingVideos, false)}
        >
          <Video /> Vidéos d’intro manquantes ({missingVideos.length})
        </Button>
      </div>

      <Tabs defaultValue="scenes">
        <TabsList>
          <TabsTrigger value="scenes">Scènes ({scenes.length})</TabsTrigger>
          <TabsTrigger value="entities">Fiches ({data.items.filter((i) => i.group === "entities").length})</TabsTrigger>
          <TabsTrigger value="maps">Cartes ({data.items.filter((i) => i.group === "maps").length})</TabsTrigger>
        </TabsList>

        <TabsContent value="scenes" className="space-y-4">
          {scenes.map((sceneId) => {
            const its = data.items.filter((i) => i.group === "scenes" && i.target.id === sceneId);
            const m = data.music.find((x) => x.sceneId === sceneId);
            const value = music[sceneId] ?? m?.musicUrl ?? "";
            return (
              <Card key={sceneId} data-testid={`scene-media-${sceneId}`}>
                <CardContent className="pt-4 space-y-3">
                  <h3 className="font-semibold">{its[0]?.label}</h3>
                  <div className="grid gap-4 md:grid-cols-2">{its.map((i) => itemCard(i, true))}</div>
                  <div className="space-y-1">
                    <p className="text-xs uppercase tracking-wide text-muted-foreground">Musique (YouTube)</p>
                    <div className="flex flex-wrap items-center gap-2">
                      <Input
                        className="h-8 max-w-md text-xs"
                        placeholder="Lien d’une vidéo ou d’une playlist YouTube"
                        aria-label={`Musique ${its[0]?.label}`}
                        value={value}
                        onChange={(e) => setMusic((x) => ({ ...x, [sceneId]: e.target.value }))}
                      />
                      <Button size="xs" variant="outline" disabled={!!value.trim() && !parseYouTube(value)} onClick={() => saveMusic(sceneId, value.trim())}>
                        Enregistrer
                      </Button>
                      {m?.musicQuery ? (
                        <a href={youTubeSearchUrl(m.musicQuery)} target="_blank" rel="noreferrer" className="text-xs text-primary underline flex items-center gap-1">
                          <ExternalLink className="size-3" /> Chercher « {m.musicQuery} »
                        </a>
                      ) : null}
                      {m?.musicUrl ? <Badge variant="secondary">enregistrée</Badge> : null}
                    </div>
                  </div>
                </CardContent>
              </Card>
            );
          })}
          {!scenes.length ? <p className="text-sm text-muted-foreground italic">Aucune scène : générez ou écrivez d’abord le scénario.</p> : null}
        </TabsContent>

        <TabsContent value="entities">
          <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-4">{data.items.filter((i) => i.group === "entities").map((i) => itemCard(i))}</div>
        </TabsContent>

        <TabsContent value="maps">
          <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">{data.items.filter((i) => i.group === "maps").map((i) => itemCard(i))}</div>
        </TabsContent>
      </Tabs>

      <AlertDialog open={!!pending} onOpenChange={(o) => !o && setPending(null)}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>{pending?.title}</AlertDialogTitle>
            <AlertDialogDescription>
              Coût estimé : environ {usd(pending?.estimate ?? 0)}
              {remaining !== null ? ` — il reste ${usd(remaining)} sur le budget.` : "."} Les médias sont générés en arrière-plan et gardés.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>Annuler</AlertDialogCancel>
            <AlertDialogAction onClick={() => pending && launch(pending)}>Lancer</AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </div>
  );
}
