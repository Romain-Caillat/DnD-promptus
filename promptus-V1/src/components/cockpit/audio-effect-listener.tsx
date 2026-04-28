"use client";

import { useEffect, useRef } from "react";
import { useQuery } from "@tanstack/react-query";
import { useAudioStore } from "@/lib/stores/audio-store";
import type { AudioAsset, SessionTimelineRow } from "@/lib/db/schema";
import type {
  Effect,
  PlayAmbienceEffect,
  PlayMusicEffect,
  PlaySoundEffect,
  ResolutionRecord,
} from "@/lib/engine/types";

/**
 * Watches the session timeline and dispatches any play_ambience / play_music /
 * play_sound primitive effects to the audio store. Only fires for timeline
 * entries created since this component mounted, so revisiting an old session
 * doesn't replay the entire history.
 */
export function AudioEffectListener({ sessionId }: { sessionId: string }) {
  const seen = useRef<Set<string>>(new Set());
  const initialized = useRef(false);

  const { data: timelineData } = useQuery({
    queryKey: ["timeline", sessionId],
    queryFn: async () => {
      const res = await fetch(`/api/sessions/${sessionId}/timeline`);
      if (!res.ok) throw new Error("Failed to load timeline");
      return res.json() as Promise<{ timeline: SessionTimelineRow[] }>;
    },
    refetchInterval: 3000,
  });

  const { data: audioData } = useQuery({
    queryKey: ["audio-assets"],
    queryFn: async () => {
      const res = await fetch("/api/audio/assets");
      if (!res.ok) throw new Error("Failed to load audio assets");
      return res.json() as Promise<{ assets: AudioAsset[] }>;
    },
  });

  const playAmbience = useAudioStore((s) => s.playAmbience);
  const playMusic = useAudioStore((s) => s.playMusic);
  const queueSound = useAudioStore((s) => s.queueSound);

  useEffect(() => {
    const rows = timelineData?.timeline ?? [];
    const assets = audioData?.assets ?? [];

    // First pass: bookmark every existing row as "already seen" without
    // dispatching, so we don't replay history on initial load.
    if (!initialized.current) {
      for (const row of rows) seen.current.add(row.id);
      initialized.current = true;
      return;
    }

    // Subsequent passes: dispatch new rows.
    for (const row of rows) {
      if (seen.current.has(row.id)) continue;
      seen.current.add(row.id);
      const r = row.resolutionRecord as ResolutionRecord | null;
      if (!r) continue;
      const fx = r.effect as Effect;
      switch (fx.type) {
        case "play_ambience": {
          const ambienceId = (fx as PlayAmbienceEffect).ambienceId;
          const asset = findAsset(assets, ambienceId, "ambience");
          if (asset) playAmbience(asset);
          break;
        }
        case "play_music": {
          const musicId = (fx as PlayMusicEffect).musicId;
          const asset = findAsset(assets, musicId, "music");
          if (asset) playMusic(asset);
          break;
        }
        case "play_sound": {
          const soundId = (fx as PlaySoundEffect).soundId;
          const asset = findAsset(assets, soundId, "sound");
          if (asset) queueSound(asset);
          break;
        }
        default:
          break;
      }
    }
  }, [timelineData, audioData, playAmbience, playMusic, queueSound]);

  return null;
}

function findAsset(
  assets: AudioAsset[],
  ref: string,
  type: AudioAsset["type"],
): AudioAsset | null {
  if (!ref) return null;
  // Match by id first, then by name (case-insensitive contains), then by tag.
  const byId = assets.find((a) => a.id === ref);
  if (byId) return byId;
  const lower = ref.toLowerCase();
  const byName = assets.find(
    (a) => a.type === type && a.name.toLowerCase().includes(lower),
  );
  if (byName) return byName;
  const byTag = assets.find(
    (a) => a.type === type && a.tags.some((t) => t.toLowerCase() === lower),
  );
  return byTag ?? null;
}
