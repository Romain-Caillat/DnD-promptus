"use client";

import { useState, useRef } from "react";
import { useQuery } from "@tanstack/react-query";
import { Play, Pause } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import type { AudioAsset } from "@/lib/db/schema";

const TYPE_LABELS: Record<AudioAsset["type"], string> = {
  ambience: "Ambiences",
  music: "Music",
  sound: "Sound effects",
};

export function AudioLibrary() {
  const { data, isLoading } = useQuery({
    queryKey: ["audio-assets-all"],
    queryFn: async () => {
      const res = await fetch("/api/audio/assets");
      if (!res.ok) throw new Error("Failed to load");
      return res.json() as Promise<{ assets: AudioAsset[] }>;
    },
  });

  const [playingId, setPlayingId] = useState<string | null>(null);
  const audioRef = useRef<HTMLAudioElement | null>(null);

  function toggle(asset: AudioAsset) {
    if (playingId === asset.id) {
      audioRef.current?.pause();
      setPlayingId(null);
      return;
    }
    audioRef.current?.pause();
    const el = new Audio(asset.filePath);
    el.volume = 0.5;
    el.addEventListener("ended", () => setPlayingId((cur) => (cur === asset.id ? null : cur)));
    el.play().catch(() => setPlayingId(null));
    audioRef.current = el;
    setPlayingId(asset.id);
  }

  if (isLoading) return <p className="text-sm text-muted-foreground">Loading library…</p>;

  const grouped: Record<AudioAsset["type"], AudioAsset[]> = {
    ambience: [],
    music: [],
    sound: [],
  };
  for (const a of data?.assets ?? []) grouped[a.type].push(a);

  return (
    <div className="space-y-6">
      {(Object.keys(grouped) as AudioAsset["type"][]).map((type) => (
        <section key={type}>
          <h2 className="text-lg font-semibold mb-2">{TYPE_LABELS[type]}</h2>
          <div className="grid gap-3 md:grid-cols-2">
            {grouped[type].length === 0 ? (
              <p className="text-sm text-muted-foreground">None.</p>
            ) : (
              grouped[type].map((a) => (
                <Card key={a.id}>
                  <CardHeader>
                    <div className="flex items-start justify-between gap-2">
                      <CardTitle className="text-base">{a.name}</CardTitle>
                      <Button
                        size="icon"
                        variant="outline"
                        onClick={() => toggle(a)}
                        aria-label={playingId === a.id ? "Pause" : "Play"}
                      >
                        {playingId === a.id ? <Pause className="size-4" /> : <Play className="size-4" />}
                      </Button>
                    </div>
                  </CardHeader>
                  <CardContent className="space-y-2">
                    <div className="flex flex-wrap gap-1">
                      {a.tags.map((t) => (
                        <Badge key={t} variant="secondary" className="text-xs">
                          {t}
                        </Badge>
                      ))}
                    </div>
                    {a.attribution ? (
                      <p className="text-[10px] text-muted-foreground italic">
                        {a.attribution}
                      </p>
                    ) : null}
                  </CardContent>
                </Card>
              ))
            )}
          </div>
        </section>
      ))}
    </div>
  );
}
