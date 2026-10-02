"use client";

import { useEffect, useRef, useState } from "react";
import { useQueryClient, type QueryKey } from "@tanstack/react-query";
import type { ClientHello, PresenceEntry, RealtimeKind, ServerMessage } from "./protocol";

export interface RealtimeState {
  connected: boolean;
  players: PresenceEntry[];
  gmOnline: boolean;
  error: string | null;
}

let configUrl: Promise<string | null> | null = null;

async function realtimeUrl(): Promise<string> {
  configUrl ??= fetch("/api/realtime/config")
    .then((r) => r.json())
    .then((j: { url: string | null }) => j.url)
    .catch(() => null);
  const url = await configUrl;
  if (url) return url;
  const proto = window.location.protocol === "https:" ? "wss" : "ws";
  return `${proto}://${window.location.hostname}:3001`;
}

/**
 * Connexion temps réel : à chaque notification, les requêtes concernées sont
 * invalidées (React Query les relit). Reconnexion automatique.
 * `keysFor` traduit un type de changement en clés de requêtes à rafraîchir.
 */
export function useRealtime(
  hello: ClientHello | null,
  keysFor: (kind: RealtimeKind) => QueryKey[],
): RealtimeState {
  const queryClient = useQueryClient();
  const [state, setState] = useState<RealtimeState>({ connected: false, players: [], gmOnline: false, error: null });
  const keysRef = useRef(keysFor);
  useEffect(() => {
    keysRef.current = keysFor;
  });
  const helloKey = hello ? JSON.stringify(hello) : null;

  useEffect(() => {
    if (!helloKey) return;
    let ws: WebSocket | null = null;
    let stopped = false;
    let retry = 0;
    let timer: ReturnType<typeof setTimeout> | undefined;

    async function connect() {
      if (stopped) return;
      ws = new WebSocket(await realtimeUrl());
      ws.onopen = () => ws?.send(helloKey!);
      ws.onmessage = (ev) => {
        const msg = JSON.parse(String(ev.data)) as ServerMessage;
        if (msg.type === "ready") {
          retry = 0;
          setState((s) => ({ ...s, connected: true, error: null }));
          // Rattrape ce qui a pu changer pendant la déconnexion.
          for (const k of ["session", "story", "timeline", "requests", "players"] as RealtimeKind[]) {
            for (const key of keysRef.current(k)) void queryClient.invalidateQueries({ queryKey: key });
          }
        } else if (msg.type === "invalidate") {
          for (const k of msg.kinds) for (const key of keysRef.current(k)) void queryClient.invalidateQueries({ queryKey: key });
        } else if (msg.type === "presence") {
          setState((s) => ({ ...s, players: msg.players, gmOnline: msg.gmOnline }));
        } else if (msg.type === "error") {
          setState((s) => ({ ...s, error: msg.message }));
        }
      };
      ws.onclose = () => {
        setState((s) => ({ ...s, connected: false }));
        if (stopped) return;
        retry = Math.min(retry + 1, 6);
        timer = setTimeout(connect, 500 * 2 ** retry);
      };
    }
    void connect();
    return () => {
      stopped = true;
      clearTimeout(timer);
      ws?.close();
    };
  }, [helloKey, queryClient]);

  return state;
}
