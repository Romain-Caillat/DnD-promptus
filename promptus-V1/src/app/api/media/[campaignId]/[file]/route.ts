import { promises as fs } from "node:fs";
import { NextRequest, NextResponse } from "next/server";
import { MEDIA_TYPES, mediaPath } from "@/lib/media/storage";

/** Sert un média généré ; les vidéos acceptent les requêtes partielles (Range). */
export async function GET(
  req: NextRequest,
  { params }: { params: Promise<{ campaignId: string; file: string }> },
) {
  const { campaignId, file } = await params;
  const full = mediaPath(campaignId, file);
  const stat = full ? await fs.stat(full).catch(() => null) : null;
  if (!full || !stat?.isFile()) return NextResponse.json({ error: { message: "Média introuvable" } }, { status: 404 });

  const type = MEDIA_TYPES[file.split(".").pop() ?? ""] ?? "application/octet-stream";
  const headers: Record<string, string> = {
    "content-type": type,
    "accept-ranges": "bytes",
    // Le nom dépend du contenu : le fichier ne change jamais.
    "cache-control": "public, max-age=31536000, immutable",
  };
  const range = req.headers.get("range")?.match(/^bytes=(\d*)-(\d*)$/);
  if (range) {
    const start = range[1] ? Number(range[1]) : Math.max(0, stat.size - Number(range[2]));
    const end = range[1] && range[2] ? Math.min(Number(range[2]), stat.size - 1) : stat.size - 1;
    if (start >= stat.size || start > end) {
      return new NextResponse(null, { status: 416, headers: { "content-range": `bytes */${stat.size}` } });
    }
    const handle = await fs.open(full, "r");
    try {
      const buf = Buffer.alloc(end - start + 1);
      await handle.read(buf, 0, buf.length, start);
      return new NextResponse(buf, {
        status: 206,
        headers: { ...headers, "content-range": `bytes ${start}-${end}/${stat.size}`, "content-length": String(buf.length) },
      });
    } finally {
      await handle.close();
    }
  }
  return new NextResponse(await fs.readFile(full), { headers: { ...headers, "content-length": String(stat.size) } });
}
