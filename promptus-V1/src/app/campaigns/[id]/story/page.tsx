import { notFound } from "next/navigation";
import Link from "next/link";
import { eq } from "drizzle-orm";
import { db } from "@/lib/db/client";
import { campaigns } from "@/lib/db/schema";
import { Button } from "@/components/ui/button";
import { RulesetProvider } from "@/components/providers/ruleset-provider";
import { StoryView } from "@/components/story/story-view";

export const dynamic = "force-dynamic";

interface Props {
  params: Promise<{ id: string }>;
}

export default async function CampaignStoryPage({ params }: Props) {
  const { id } = await params;
  const [campaign] = await db.select().from(campaigns).where(eq(campaigns.id, id));
  if (!campaign) notFound();

  return (
    <main className="flex-1 px-6 py-8 max-w-5xl mx-auto w-full">
      <Button asChild variant="ghost" size="sm" className="mb-2 -ml-2">
        <Link href={`/campaigns/${id}`}>← {campaign.name}</Link>
      </Button>
      <h1 className="text-3xl font-bold mb-1">Scénario</h1>
      <p className="text-muted-foreground mb-8">
        Bible, menaces, scènes, révélations et cartes de la campagne.
      </p>
      <RulesetProvider campaignId={id}>
        <StoryView campaignId={id} />
      </RulesetProvider>
    </main>
  );
}
