"use client";

import { useQuery } from "@tanstack/react-query";
import { Volume2, VolumeX, Music, Wind, Square } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { Slider } from "@/components/ui/slider";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { useAudioStore } from "@/lib/stores/audio-store";
import type { AudioAsset } from "@/lib/db/schema";

export function AudioControls() {
  const { data } = useQuery({
    queryKey: ["audio-assets"],
    queryFn: async () => {
      const res = await fetch("/api/audio/assets");
      if (!res.ok) throw new Error("Impossible de charger les pistes audio");
      return res.json() as Promise<{ assets: AudioAsset[] }>;
    },
  });

  const ambiences = (data?.assets ?? []).filter((a) => a.type === "ambience");
  const musics = (data?.assets ?? []).filter((a) => a.type === "music");

  const currentAmbience = useAudioStore((s) => s.currentAmbience);
  const currentMusic = useAudioStore((s) => s.currentMusic);
  const ambienceVolume = useAudioStore((s) => s.ambienceVolume);
  const musicVolume = useAudioStore((s) => s.musicVolume);
  const effectsVolume = useAudioStore((s) => s.effectsVolume);
  const playAmbience = useAudioStore((s) => s.playAmbience);
  const playMusic = useAudioStore((s) => s.playMusic);
  const stopAll = useAudioStore((s) => s.stopAll);
  const setVolumes = useAudioStore((s) => s.setVolumes);

  return (
    <Card>
      <CardContent className="pt-4 space-y-3">
        <div className="flex items-center justify-between">
          <h3 className="text-sm font-semibold uppercase tracking-wide flex items-center gap-1">
            <Volume2 className="size-4" /> Audio
          </h3>
          <Button size="sm" variant="ghost" onClick={stopAll} disabled={!currentAmbience && !currentMusic}>
            <VolumeX className="size-4" /> Tout couper
          </Button>
        </div>

        <Row icon={<Wind className="size-4 text-muted-foreground" />} label="Ambiance">
          <Select
            value={currentAmbience?.id ?? ""}
            onValueChange={(v) => {
              const a = ambiences.find((x) => x.id === v);
              playAmbience(a ?? null);
            }}
          >
            <SelectTrigger className="h-8 text-xs">
              <SelectValue placeholder="Aucune" />
            </SelectTrigger>
            <SelectContent>
              {ambiences.map((a) => (
                <SelectItem key={a.id} value={a.id}>
                  {a.name}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
          <SliderRow value={ambienceVolume} onChange={(v) => setVolumes({ ambienceVolume: v })} />
        </Row>

        <Row icon={<Music className="size-4 text-muted-foreground" />} label="Musique">
          <Select
            value={currentMusic?.id ?? ""}
            onValueChange={(v) => {
              const m = musics.find((x) => x.id === v);
              playMusic(m ?? null);
            }}
          >
            <SelectTrigger className="h-8 text-xs">
              <SelectValue placeholder="Aucune" />
            </SelectTrigger>
            <SelectContent>
              {musics.map((a) => (
                <SelectItem key={a.id} value={a.id}>
                  {a.name}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
          <SliderRow value={musicVolume} onChange={(v) => setVolumes({ musicVolume: v })} />
        </Row>

        <Row icon={<Square className="size-4 text-muted-foreground" />} label="Bruitages">
          <SliderRow value={effectsVolume} onChange={(v) => setVolumes({ effectsVolume: v })} />
        </Row>
      </CardContent>
    </Card>
  );
}

function Row({
  icon,
  label,
  children,
}: {
  icon: React.ReactNode;
  label: string;
  children: React.ReactNode;
}) {
  return (
    <div className="space-y-1">
      <div className="flex items-center gap-2">
        {icon}
        <span className="text-xs uppercase tracking-wide text-muted-foreground">{label}</span>
      </div>
      <div className="pl-6 space-y-1">{children}</div>
    </div>
  );
}

function SliderRow({
  value,
  onChange,
}: {
  value: number;
  onChange: (v: number) => void;
}) {
  return (
    <div className="flex items-center gap-2">
      <Slider
        value={[Math.round(value * 100)]}
        max={100}
        step={1}
        onValueChange={(arr) => onChange((arr[0] ?? 0) / 100)}
      />
      <span className="text-[10px] text-muted-foreground w-8 text-right tabular-nums">
        {Math.round(value * 100)}%
      </span>
    </div>
  );
}
