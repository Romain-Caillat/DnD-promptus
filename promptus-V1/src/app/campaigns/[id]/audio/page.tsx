import { notFound } from "next/navigation";
import Link from "next/link";
import { db } from "@/lib/db/client";
import { campaigns } from "@/lib/db/schema";
import { eq } from "drizzle-orm";
import { Button } from "@/components/ui/button";
import { AudioLibrary } from "@/components/audio/audio-library";

export const dynamic = "force-dynamic";

interface Props {
  params: Promise<{ id: string }>;
}

export default async function AudioLibraryPage({ params }: Props) {
  const { id } = await params;
  const [campaign] = await db.select().from(campaigns).where(eq(campaigns.id, id));
  if (!campaign) notFound();

  return (
    <main className="flex-1 px-6 py-8 max-w-5xl mx-auto w-full">
      <Button asChild variant="ghost" size="sm" className="mb-2 -ml-2">
        <Link href={`/campaigns/${campaign.id}`}>← {campaign.name}</Link>
      </Button>
      <h1 className="text-3xl font-bold mb-1">Bibliothèque audio</h1>
      <p className="text-muted-foreground mb-8">
        Ambiances, musiques et bruitages libres de droits, disponibles dans toutes
        vos campagnes. Cliquez sur lecture pour écouter.
      </p>
      <AudioLibrary />
    </main>
  );
}
