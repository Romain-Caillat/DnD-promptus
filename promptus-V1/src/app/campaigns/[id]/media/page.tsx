import { notFound } from "next/navigation";
import Link from "next/link";
import { eq } from "drizzle-orm";
import { db } from "@/lib/db/client";
import { campaigns } from "@/lib/db/schema";
import { Button } from "@/components/ui/button";
import { MediaStudio } from "@/components/media/media-studio";

export const dynamic = "force-dynamic";

export default async function CampaignMediaPage({ params }: { params: Promise<{ id: string }> }) {
  const { id } = await params;
  const [campaign] = await db.select({ name: campaigns.name }).from(campaigns).where(eq(campaigns.id, id));
  if (!campaign) notFound();
  return (
    <main className="flex-1 px-6 py-8 max-w-6xl mx-auto w-full">
      <Button asChild variant="ghost" size="sm" className="mb-2 -ml-2">
        <Link href={`/campaigns/${id}`}>← {campaign.name}</Link>
      </Button>
      <h1 className="text-3xl font-bold mb-1">Médias</h1>
      <p className="text-muted-foreground mb-8">
        Tout est généré à l’avance et gardé : rien n’est produit pendant la partie. Le coût est estimé avant chaque lot.
      </p>
      <MediaStudio campaignId={id} />
    </main>
  );
}
