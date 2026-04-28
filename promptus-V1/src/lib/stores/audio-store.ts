import { create } from "zustand";
import { persist } from "zustand/middleware";
import type { AudioAsset } from "@/lib/db/schema";

interface AudioState {
  ambienceVolume: number; // 0..1
  musicVolume: number;
  effectsVolume: number;
  currentAmbience: AudioAsset | null;
  currentMusic: AudioAsset | null;
  /** Sound IDs that the AudioPlayer should fire-and-forget play (queued). */
  pendingEffects: { id: string; asset: AudioAsset }[];
  setVolumes: (next: { ambienceVolume?: number; musicVolume?: number; effectsVolume?: number }) => void;
  playAmbience: (a: AudioAsset | null) => void;
  playMusic: (a: AudioAsset | null) => void;
  queueSound: (a: AudioAsset) => void;
  consumeSound: (id: string) => void;
  stopAll: () => void;
}

export const useAudioStore = create<AudioState>()(
  persist(
    (set) => ({
      ambienceVolume: 0.4,
      musicVolume: 0.5,
      effectsVolume: 0.7,
      currentAmbience: null,
      currentMusic: null,
      pendingEffects: [],
      setVolumes: (next) => set(next),
      playAmbience: (a) => set({ currentAmbience: a }),
      playMusic: (a) => set({ currentMusic: a }),
      queueSound: (a) =>
        set((s) => ({
          pendingEffects: [
            ...s.pendingEffects,
            { id: `${a.id}-${Date.now()}`, asset: a },
          ],
        })),
      consumeSound: (id) =>
        set((s) => ({ pendingEffects: s.pendingEffects.filter((p) => p.id !== id) })),
      stopAll: () =>
        set({
          currentAmbience: null,
          currentMusic: null,
          pendingEffects: [],
        }),
    }),
    {
      name: "promptus-audio-prefs",
      // Only persist user preferences, not the runtime queues.
      partialize: (state) => ({
        ambienceVolume: state.ambienceVolume,
        musicVolume: state.musicVolume,
        effectsVolume: state.effectsVolume,
      }),
    },
  ),
);
