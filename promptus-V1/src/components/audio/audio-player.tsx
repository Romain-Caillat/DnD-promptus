"use client";

import { useEffect, useRef } from "react";
import { useAudioStore } from "@/lib/stores/audio-store";

/**
 * Mounted once at the app root. Manages 3 categories of audio:
 *  - 1 looping ambience element
 *  - 1 looping music element
 *  - N fire-and-forget effect elements created on demand
 */
export function AudioPlayer() {
  const ambienceRef = useRef<HTMLAudioElement | null>(null);
  const musicRef = useRef<HTMLAudioElement | null>(null);

  const ambience = useAudioStore((s) => s.currentAmbience);
  const music = useAudioStore((s) => s.currentMusic);
  const ambienceVolume = useAudioStore((s) => s.ambienceVolume);
  const musicVolume = useAudioStore((s) => s.musicVolume);
  const effectsVolume = useAudioStore((s) => s.effectsVolume);
  const pendingEffects = useAudioStore((s) => s.pendingEffects);
  const consumeSound = useAudioStore((s) => s.consumeSound);

  // Lazily create the audio elements (browsers require a user gesture before
  // play() succeeds, but creation is fine).
  useEffect(() => {
    if (!ambienceRef.current) {
      const el = new Audio();
      el.loop = true;
      el.preload = "auto";
      ambienceRef.current = el;
    }
    if (!musicRef.current) {
      const el = new Audio();
      el.loop = true;
      el.preload = "auto";
      musicRef.current = el;
    }
  }, []);

  // Sync ambience element with store state.
  useEffect(() => {
    const el = ambienceRef.current;
    if (!el) return;
    if (!ambience) {
      el.pause();
      el.removeAttribute("src");
      return;
    }
    if (el.src !== ambience.filePath) {
      el.src = ambience.filePath;
    }
    el.volume = ambienceVolume;
    el.play().catch(() => {
      // Browser may block autoplay until interaction; safe to ignore.
    });
  }, [ambience, ambienceVolume]);

  // Sync music element.
  useEffect(() => {
    const el = musicRef.current;
    if (!el) return;
    if (!music) {
      el.pause();
      el.removeAttribute("src");
      return;
    }
    if (el.src !== music.filePath) {
      el.src = music.filePath;
    }
    el.volume = musicVolume;
    el.play().catch(() => {});
  }, [music, musicVolume]);

  // Volume tracking when volumes change (only).
  useEffect(() => {
    if (ambienceRef.current) ambienceRef.current.volume = ambienceVolume;
  }, [ambienceVolume]);
  useEffect(() => {
    if (musicRef.current) musicRef.current.volume = musicVolume;
  }, [musicVolume]);

  // Fire-and-forget sound effects.
  useEffect(() => {
    if (pendingEffects.length === 0) return;
    for (const fx of pendingEffects) {
      const el = new Audio(fx.asset.filePath);
      el.volume = effectsVolume;
      el.play().catch(() => {});
      // Remove from queue immediately; let it play out on its own.
      consumeSound(fx.id);
    }
  }, [pendingEffects, effectsVolume, consumeSound]);

  return null;
}
