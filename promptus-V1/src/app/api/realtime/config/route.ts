import { NextResponse } from "next/server";

// Lue à l'exécution (variable d'environnement du conteneur), jamais figée au build.
export const dynamic = "force-dynamic";

/**
 * Adresse publique du serveur temps réel. Sans REALTIME_PUBLIC_URL, le client
 * la déduit de l'adresse de la page (même hôte, port 3001).
 */
export async function GET() {
  return NextResponse.json({ url: process.env.REALTIME_PUBLIC_URL ?? null });
}
