"use client";

import { useEffect, useRef, useState } from "react";
import { Music, Volume2, VolumeX } from "lucide-react";
import { Button } from "@/components/ui/button";
import { musicPosition, type MusicState } from "@/lib/media/youtube";
import { cn } from "@/lib/utils";

// Lecteur YouTube visible (exigé par les conditions de YouTube), calé sur
// l'état partagé. Il démarre en muet — les navigateurs bloquent le son sans
// geste de l'utilisateur — et propose « Activer le son ».

interface YTPlayer {
  loadVideoById(o: { videoId: string; startSeconds?: number }): void;
  cueVideoById(o: { videoId: string; startSeconds?: number }): void;
  loadPlaylist(o: { list: string; listType: "playlist"; index?: number; startSeconds?: number }): void;
  cuePlaylist(o: { list: string; listType: "playlist"; index?: number; startSeconds?: number }): void;
  playVideo(): void;
  pauseVideo(): void;
  stopVideo(): void;
  seekTo(s: number, allowSeekAhead: boolean): void;
  getCurrentTime(): number;
  getDuration(): number;
  mute(): void;
  unMute(): void;
  isMuted(): boolean;
  setVolume(v: number): void;
  destroy(): void;
}

interface YTNamespace {
  Player: new (
    el: HTMLElement,
    opts: {
      width?: string | number;
      height?: string | number;
      playerVars?: Record<string, number | string>;
      events?: { onReady?: () => void; onStateChange?: (e: { data: number }) => void };
    },
  ) => YTPlayer;
}

declare global {
  interface Window {
    YT?: YTNamespace;
    onYouTubeIframeAPIReady?: () => void;
  }
}

let apiPromise: Promise<YTNamespace> | null = null;
function loadApi(): Promise<YTNamespace> {
  if (window.YT?.Player) return Promise.resolve(window.YT);
  apiPromise ??= new Promise((resolve) => {
    const previous = window.onYouTubeIframeAPIReady;
    window.onYouTubeIframeAPIReady = () => {
      previous?.();
      resolve(window.YT!);
    };
    const s = document.createElement("script");
    s.src = "https://www.youtube.com/iframe_api";
    document.head.appendChild(s);
  });
  return apiPromise;
}

const ENDED = 0;

export function YouTubeMusic({
  music,
  serverTime,
  className,
}: {
  music: MusicState | null;
  /** Heure serveur reçue avec l'état : corrige le décalage d'horloge. */
  serverTime: number;
  className?: string;
}) {
  const host = useRef<HTMLDivElement>(null);
  const player = useRef<YTPlayer | null>(null);
  const ready = useRef(false);
  const musicRef = useRef(music);
  const [soundOn, setSoundOn] = useState(false);
  const [volume, setVolume] = useState(60);
  // Décalage d'horloge client/serveur, recalculé à chaque nouvel état reçu
  // (cet effet passe avant celui qui cale la lecture).
  const skewRef = useRef(0);
  useEffect(() => {
    skewRef.current = serverTime - Date.now();
  }, [serverTime]);
  useEffect(() => {
    musicRef.current = music;
  });

  const expected = (p: YTPlayer, m: MusicState) => {
    const pos = musicPosition(m, Date.now() + skewRef.current);
    const dur = m.playlistId ? 0 : p.getDuration();
    return dur > 0 ? pos % dur : pos;
  };

  const sync = () => {
    const p = player.current;
    const m = musicRef.current;
    if (!p || !ready.current) return;
    if (!m) return p.stopVideo();
    const start = Math.floor(expected(p, m));
    if (m.playlistId) {
      const o = { list: m.playlistId, listType: "playlist" as const, startSeconds: start };
      if (m.playing) p.loadPlaylist(o);
      else p.cuePlaylist(o);
    } else if (m.videoId) {
      if (m.playing) p.loadVideoById({ videoId: m.videoId, startSeconds: start });
      else p.cueVideoById({ videoId: m.videoId, startSeconds: start });
    }
  };

  // Création du lecteur à la première musique.
  const hasMusic = !!music;
  useEffect(() => {
    if (!hasMusic || player.current || !host.current) return;
    let cancelled = false;
    void loadApi().then((YT) => {
      if (cancelled || !host.current) return;
      const el = document.createElement("div");
      host.current.appendChild(el);
      player.current = new YT.Player(el, {
        width: "100%",
        height: "100%",
        playerVars: { autoplay: 1, controls: 0, disablekb: 1, modestbranding: 1, playsinline: 1, rel: 0 },
        events: {
          onReady: () => {
            ready.current = true;
            player.current?.mute();
            player.current?.setVolume(volume);
            sync();
          },
          onStateChange: (e) => {
            // Ambiance : une vidéo seule tourne en boucle.
            const m = musicRef.current;
            if (e.data === ENDED && m?.playing && !m.playlistId) {
              player.current?.seekTo(0, true);
              player.current?.playVideo();
            }
          },
        },
      });
    });
    return () => {
      cancelled = true;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [hasMusic]);

  // Changement de morceau, pause, reprise.
  const syncKey = music ? `${music.videoId}|${music.playlistId}|${music.startedAt}|${music.offsetSec}|${music.playing}` : "none";
  useEffect(() => {
    sync();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [syncKey]);

  // Correction de dérive (publicités, mise en mémoire tampon).
  useEffect(() => {
    const t = setInterval(() => {
      const p = player.current;
      const m = musicRef.current;
      if (!p || !ready.current || !m?.playing || m.playlistId) return;
      const target = expected(p, m);
      if (Math.abs(p.getCurrentTime() - target) > 3) p.seekTo(target, true);
    }, 15_000);
    return () => clearInterval(t);
  }, []);

  useEffect(
    () => () => {
      player.current?.destroy();
      player.current = null;
    },
    [],
  );

  if (!music) return null;
  return (
    <div className={cn("space-y-2", className)} data-testid="youtube-music">
      <div className="flex items-center justify-between gap-2 text-xs">
        <span className="flex items-center gap-1 truncate text-muted-foreground">
          <Music className="size-3 shrink-0" />
          <span className="truncate">{music.title ?? "Musique d’ambiance"}</span>
          {!music.playing ? " (en pause)" : null}
        </span>
        <div className="flex items-center gap-1">
          <input
            type="range"
            min={0}
            max={100}
            value={volume}
            aria-label="Volume"
            className="w-20"
            onChange={(e) => {
              setVolume(Number(e.target.value));
              player.current?.setVolume(Number(e.target.value));
            }}
          />
          <Button
            size="xs"
            variant={soundOn ? "ghost" : "default"}
            onClick={() => {
              if (soundOn) player.current?.mute();
              else {
                player.current?.unMute();
                player.current?.playVideo();
                if (musicRef.current && !musicRef.current.playing) player.current?.pauseVideo();
              }
              setSoundOn(!soundOn);
            }}
          >
            {soundOn ? <Volume2 /> : <VolumeX />} {soundOn ? "Couper" : "Activer le son"}
          </Button>
        </div>
      </div>
      {/* Lecteur visible, au moins 200 px de haut (conditions de YouTube). */}
      <div ref={host} className="h-[200px] w-full overflow-hidden rounded bg-black" />
    </div>
  );
}
