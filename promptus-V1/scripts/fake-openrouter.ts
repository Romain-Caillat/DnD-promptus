// Faux serveur OpenRouter (API compatible) pour essayer la génération sans
// clé ni réseau : OPENROUTER_BASE_URL=http://localhost:4010/api/v1
// Lancement : pnpm tsx scripts/fake-openrouter.ts [port] [délai_ms]

import { readFileSync } from "node:fs";
import { createServer } from "node:http";
import path from "node:path";
import { crc32, deflateSync } from "node:zlib";
import { FakeLlm } from "./fake-llm";

/** PNG uni (couleur tirée du prompt) : de quoi vérifier stockage et affichage. */
function pngDataUrl(seed: string, w = 96, h = 64): string {
  let hash = 0;
  for (const ch of seed) hash = (hash * 31 + ch.charCodeAt(0)) >>> 0;
  const [r, g, b] = [hash & 255, (hash >> 8) & 255, (hash >> 16) & 255];
  const raw = Buffer.alloc((w * 3 + 1) * h);
  for (let y = 0; y < h; y++) {
    raw[y * (w * 3 + 1)] = 0;
    for (let x = 0; x < w; x++) {
      const o = y * (w * 3 + 1) + 1 + x * 3;
      const shade = 0.6 + (0.4 * (x + y)) / (w + h);
      raw[o] = r * shade;
      raw[o + 1] = g * shade;
      raw[o + 2] = b * shade;
    }
  }
  const chunk = (type: string, data: Buffer) => {
    const len = Buffer.alloc(4);
    len.writeUInt32BE(data.length);
    const td = Buffer.concat([Buffer.from(type), data]);
    const crc = Buffer.alloc(4);
    crc.writeUInt32BE(crc32(td));
    return Buffer.concat([len, td, crc]);
  };
  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(w, 0);
  ihdr.writeUInt32BE(h, 4);
  ihdr.set([8, 2, 0, 0, 0], 8);
  const png = Buffer.concat([
    Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]),
    chunk("IHDR", ihdr),
    chunk("IDAT", deflateSync(raw)),
    chunk("IEND", Buffer.alloc(0)),
  ]);
  return `data:image/png;base64,${png.toString("base64")}`;
}

// Lancé depuis la racine du projet.
const VIDEO = readFileSync(path.join(process.cwd(), "scripts", "fixtures", "demo-intro.mp4"));

const port = Number(process.argv[2] ?? 4010);
const delayMs = Number(process.argv[3] ?? 800);
const llm = new FakeLlm({ breakClues: true, costPerCall: 0.012 });

createServer(async (req, res) => {
  const send = (status: number, body: unknown) => {
    res.writeHead(status, { "content-type": "application/json" });
    res.end(JSON.stringify(body));
  };
  if (req.method === "GET" && req.url?.endsWith("/models")) {
    return send(200, {
      data: [
        { id: "fake/demo", name: "Faux modèle (démo)", context_length: 200000, pricing: { prompt: "0.000003", completion: "0.000015" }, architecture: { output_modalities: ["text"] } },
        { id: "fake/image", name: "Faux modèle d’images", context_length: 32000, pricing: { prompt: "0", completion: "0" }, architecture: { output_modalities: ["image", "text"] } },
        { id: "fake/video", name: "Faux modèle vidéo", context_length: 32000, pricing: { prompt: "0", completion: "0" }, architecture: { output_modalities: ["video"] } },
      ],
    });
  }
  if (req.method === "GET" && req.url?.endsWith("/files/intro.mp4")) {
    res.writeHead(200, { "content-type": "video/mp4" });
    return res.end(VIDEO);
  }
  if (req.method === "POST" && req.url?.endsWith("/chat/completions")) {
    let raw = "";
    for await (const chunk of req) raw += chunk;
    const body = JSON.parse(raw);
    await new Promise((r) => setTimeout(r, delayMs));
    const prompt = String(body.messages?.at(-1)?.content ?? "");
    if (body.modalities?.includes("image")) {
      if (prompt.includes("ÉCHEC")) return send(400, { error: { message: "contenu refusé (test)" } });
      return send(200, {
        model: body.model,
        choices: [{ message: { content: "", images: [{ type: "image_url", image_url: { url: pngDataUrl(prompt) } }] } }],
        usage: { cost: 0.03 },
      });
    }
    if (body.modalities?.includes("video")) {
      // Fichier à télécharger (l'autre forme possible d'une réponse média).
      return send(200, {
        model: body.model,
        choices: [{ message: { content: "", videos: [{ type: "video_url", video_url: { url: `http://localhost:${port}/files/intro.mp4` } }] } }],
        usage: { cost: 0.4 },
      });
    }
    try {
      const out = await llm.complete({ messages: body.messages, model: body.model });
      return send(200, {
        model: out.model,
        choices: [{ message: { content: out.text }, finish_reason: "stop" }],
        usage: { prompt_tokens: out.usage.promptTokens, completion_tokens: out.usage.completionTokens, cost: out.usage.costUsd },
      });
    } catch (e) {
      return send(400, { error: { message: String(e) } });
    }
  }
  send(404, { error: { message: "not found" } });
}).listen(port, () => console.log(`[fake-openrouter] http://localhost:${port}/api/v1`));
