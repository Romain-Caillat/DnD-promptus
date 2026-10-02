// Faux serveur OpenRouter (API compatible) pour essayer la génération sans
// clé ni réseau : OPENROUTER_BASE_URL=http://localhost:4010/api/v1
// Lancement : pnpm tsx scripts/fake-openrouter.ts [port] [délai_ms]

import { createServer } from "node:http";
import { FakeLlm } from "./fake-llm";

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
      ],
    });
  }
  if (req.method === "POST" && req.url?.endsWith("/chat/completions")) {
    let raw = "";
    for await (const chunk of req) raw += chunk;
    const body = JSON.parse(raw);
    await new Promise((r) => setTimeout(r, delayMs));
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
