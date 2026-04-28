import { notFound } from "next/navigation";
import Link from "next/link";
import { db } from "@/lib/db/client";
import { campaigns } from "@/lib/db/schema";
import { eq } from "drizzle-orm";
import { EntitiesView } from "@/components/entities/entities-view";
import { Button } from "@/components/ui/button";

export const dynamic = "force-dynamic";

interface Props {
  params: Promise<{ id: string }>;
}

export default async function CampaignEntitiesPage({ params }: Props) {
  const { id } = await params;
  const [campaign] = await db.select().from(campaigns).where(eq(campaigns.id, id));
  if (!campaign) notFound();

  return (
    <main className="flex-1 px-6 py-8 max-w-7xl mx-auto w-full">
      <div className="flex items-center justify-between mb-6">
        <div>
          <Button asChild variant="ghost" size="sm" className="mb-2 -ml-2">
            <Link href="/campaigns">← Campaigns</Link>
          </Button>
          <h1 className="text-3xl font-bold">{campaign.name}</h1>
          <p className="text-muted-foreground mt-1">Entities — the inhabitants and props of your world</p>
        </div>
      </div>

      <EntitiesView campaignId={campaign.id} />
    </main>
  );
}
