// Protocole temps réel partagé entre l'API, le serveur WebSocket et les clients.
// Le serveur ne transporte aucune donnée de jeu : seulement « telle partie a
// changé » et la présence des joueurs. Les clients relisent via l'API.

export const REALTIME_CHANNEL = "promptus_events";

export type RealtimeKind = "session" | "story" | "timeline" | "requests" | "players";

export type ClientHello =
  | { type: "hello"; role: "gm"; sessionId: string }
  | { type: "hello"; role: "player"; inviteCode: string; token: string };

export interface PresenceEntry {
  playerId: string;
  name: string;
}

export type ServerMessage =
  | { type: "ready"; sessionId: string }
  | { type: "invalidate"; kinds: RealtimeKind[] }
  | { type: "presence"; players: PresenceEntry[]; gmOnline: boolean }
  | { type: "error"; message: string };
