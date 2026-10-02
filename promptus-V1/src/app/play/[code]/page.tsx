import { PlayerApp } from "@/components/play/player-app";

export const metadata = { title: "Promptus — Partie" };

export default async function PlayPage({ params }: { params: Promise<{ code: string }> }) {
  const { code } = await params;
  return <PlayerApp code={code} />;
}
