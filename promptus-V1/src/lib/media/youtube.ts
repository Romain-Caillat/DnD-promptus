// Musique d'ambiance YouTube : analyse des liens et synchronisation de la
// lecture entre le MJ et les joueurs. Fonctions pures.

export interface YouTubeRef {
  videoId?: string;
  playlistId?: string;
}

/** État de lecture partagé (dans l'état du monde). */
export interface MusicState extends YouTubeRef {
  title?: string;
  /** Heure serveur (ms) du dernier départ ou de la dernière reprise. */
  startedAt: number;
  /** Position (s) au moment de startedAt. */
  offsetSec: number;
  playing: boolean;
}

const ID = /^[A-Za-z0-9_-]{11}$/;
const LIST = /^[A-Za-z0-9_-]{10,64}$/;

/** Lien YouTube (vidéo, playlist, youtu.be, shorts, embed, music) → identifiants. */
export function parseYouTube(input: string): YouTubeRef | null {
  const raw = input.trim();
  if (ID.test(raw)) return { videoId: raw };
  let url: URL;
  try {
    url = new URL(raw.startsWith("http") ? raw : `https://${raw}`);
  } catch {
    return null;
  }
  const host = url.hostname.replace(/^(www|m|music)\./, "");
  let videoId: string | undefined;
  if (host === "youtu.be") videoId = url.pathname.slice(1).split("/")[0];
  else if (host === "youtube.com" || host === "youtube-nocookie.com") {
    videoId = url.searchParams.get("v") ?? url.pathname.match(/^\/(?:embed|shorts|live)\/([^/]+)/)?.[1] ?? undefined;
  } else return null;
  const list = url.searchParams.get("list") ?? undefined;
  const ref: YouTubeRef = {
    ...(videoId && ID.test(videoId) ? { videoId } : {}),
    ...(list && LIST.test(list) ? { playlistId: list } : {}),
  };
  return ref.videoId || ref.playlistId ? ref : null;
}

/** Position de lecture attendue (s) à l'instant `now` (heure serveur). */
export function musicPosition(m: MusicState, now: number): number {
  return m.playing ? m.offsetSec + Math.max(0, now - m.startedAt) / 1000 : m.offsetSec;
}

/** Lien de recherche YouTube pour une ambiance proposée par le LLM. */
export function youTubeSearchUrl(query: string): string {
  return `https://www.youtube.com/results?search_query=${encodeURIComponent(query)}`;
}
