"use client";

import { useState } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { ExternalLink, Pause, Play, Square } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { YouTubeMusic } from "@/components/media/youtube-music";
import type { CampaignStory } from "@/lib/engine/story";
import type { WorldState } from "@/lib/engine/world";
import { youTubeSearchUrl } from "@/lib/media/youtube";

/** Musique d'ambiance YouTube, jouée en même temps chez tous les joueurs. */
export function MusicPanel({ sessionId }: { sessionId: string }) {
  const queryClient = useQueryClient();
  const [url, setUrl] = useState("");
  const [busy, setBusy] = useState(false);
  const key = ["session-story", sessionId];
  const { data } = useQuery({
    queryKey: key,
    queryFn: async () => {
      const res = await fetch(`/api/sessions/${sessionId}/story`);
      if (!res.ok) throw new Error("Impossible de charger le scénario");
      return (await res.json()) as { story: CampaignStory; world: WorldState; serverTime: number };
    },
  });
  const music = data?.world.music ?? null;
  const scene = data?.story.scenes.find((s) => s.id === data.world.currentSceneId);

  async function send(body: object) {
    setBusy(true);
    try {
      const res = await fetch(`/api/sessions/${sessionId}/music`, {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify(body),
      });
      const json = await res.json();
      if (!res.ok) throw new Error(json.error?.message ?? "Action impossible");
      void queryClient.invalidateQueries({ queryKey: key });
    } catch (e) {
      toast.error(e instanceof Error ? e.message : "Action impossible");
    } finally {
      setBusy(false);
    }
  }

  return (
    <Card data-testid="music-panel">
      <CardContent className="pt-4 space-y-3">
        <h3 className="text-sm font-semibold uppercase tracking-wide">Musique (YouTube)</h3>
        {scene?.media?.musicUrl ? (
          <Button
            size="sm"
            variant="outline"
            className="w-full justify-start"
            disabled={busy}
            onClick={() => send({ action: "play", url: scene.media!.musicUrl, title: `Ambiance : ${scene.title}` })}
          >
            <Play /> Ambiance de la scène
          </Button>
        ) : scene?.media?.musicQuery ? (
          <a
            href={youTubeSearchUrl(scene.media.musicQuery)}
            target="_blank"
            rel="noreferrer"
            className="text-xs text-primary underline flex items-center gap-1"
          >
            <ExternalLink className="size-3" /> Chercher « {scene.media.musicQuery} »
          </a>
        ) : null}
        <form
          className="flex gap-1"
          onSubmit={(e) => {
            e.preventDefault();
            if (url.trim()) void send({ action: "play", url: url.trim() }).then(() => setUrl(""));
          }}
        >
          <Input value={url} onChange={(e) => setUrl(e.target.value)} placeholder="Lien YouTube…" aria-label="Lien YouTube" className="h-8 text-xs" />
          <Button size="sm" type="submit" disabled={busy || !url.trim()}>
            Jouer
          </Button>
        </form>
        {music ? (
          <div className="flex gap-1">
            {music.playing ? (
              <Button size="xs" variant="outline" disabled={busy} onClick={() => send({ action: "pause" })}>
                <Pause /> Pause
              </Button>
            ) : (
              <Button size="xs" variant="outline" disabled={busy} onClick={() => send({ action: "resume" })}>
                <Play /> Reprendre
              </Button>
            )}
            <Button size="xs" variant="ghost" disabled={busy} onClick={() => send({ action: "stop" })}>
              <Square /> Arrêter
            </Button>
          </div>
        ) : (
          <p className="text-xs text-muted-foreground">Les joueurs entendent la même musique, au même moment.</p>
        )}
        <YouTubeMusic music={music} serverTime={data?.serverTime ?? 0} />
      </CardContent>
    </Card>
  );
}
