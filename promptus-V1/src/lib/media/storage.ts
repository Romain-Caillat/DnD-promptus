import "server-only";
import { createHash } from "node:crypto";
import { promises as fs } from "node:fs";
import path from "node:path";

// Médias générés (images, vidéos) : stockés hors de `public/` — Next ne sert
// pas les fichiers ajoutés après le démarrage — et servis par /api/media.

export const MEDIA_TYPES: Record<string, string> = {
  png: "image/png",
  jpg: "image/jpeg",
  jpeg: "image/jpeg",
  webp: "image/webp",
  gif: "image/gif",
  mp4: "video/mp4",
  webm: "video/webm",
};

// Chemins calculés à l'exécution : exclus du traçage du bundle (turbopackIgnore).
export function mediaDir(): string {
  return process.env.MEDIA_DIR ?? path.join(/*turbopackIgnore: true*/ process.cwd(), "data", "media");
}

const SAFE = /^[a-zA-Z0-9_-]+$/;
const SAFE_FILE = /^[a-f0-9]{8,64}\.[a-z0-9]{2,5}$/;

/** Extension d'après le type MIME (data URL, en-tête HTTP). */
export function extFromMime(mime: string): string | null {
  const m = mime.toLowerCase().split(";")[0].trim();
  return Object.entries(MEDIA_TYPES).find(([, t]) => t === m)?.[0] ?? null;
}

/** Enregistre un média ; le nom dépend du contenu (même fichier = même URL). */
export async function saveMedia(campaignId: string, data: Buffer, ext: string): Promise<string> {
  if (!SAFE.test(campaignId)) throw new Error("Identifiant de campagne invalide");
  if (!MEDIA_TYPES[ext]) throw new Error(`Type de média non pris en charge : ${ext}`);
  const name = `${createHash("sha256").update(data).digest("hex").slice(0, 24)}.${ext}`;
  const dir = path.join(/*turbopackIgnore: true*/ mediaDir(), campaignId);
  await fs.mkdir(dir, { recursive: true });
  await fs.writeFile(path.join(/*turbopackIgnore: true*/ dir, name), data);
  return `/api/media/${campaignId}/${name}`;
}

/** Chemin d'un média, ou null si le nom est suspect. */
export function mediaPath(campaignId: string, file: string): string | null {
  if (!SAFE.test(campaignId) || !SAFE_FILE.test(file)) return null;
  return path.join(/*turbopackIgnore: true*/ mediaDir(), campaignId, file);
}

/** Décode une data URL (base64) renvoyée par un modèle. */
export function decodeDataUrl(url: string): { data: Buffer; ext: string } | null {
  const m = url.match(/^data:([^;,]+)(?:;[^,]*)?;base64,([\s\S]+)$/);
  if (!m) return null;
  const ext = extFromMime(m[1]);
  return ext ? { data: Buffer.from(m[2], "base64"), ext } : null;
}
