import { AiSettingsCard } from "@/components/campaigns/ai-settings-card";
import { defaultImageModel, defaultModel, defaultVideoModel } from "@/lib/ai/llm";
import { notFound } from "next/navigation";
import Link from "next/link";
import { db } from "@/lib/db/client";
import { campaigns } from "@/lib/db/schema";
import { eq } from "drizzle-orm";
import { Button } from "@/components/ui/button";
import { CampaignSettingsForm } from "@/components/campaigns/campaign-settings-form";
import type { StyleGuide } from "@/lib/engine/types";

export const dynamic = "force-dynamic";

interface Props {
  params: Promise<{ id: string }>;
}

export default async function CampaignSettingsPage({ params }: Props) {
  const { id } = await params;
  const [campaign] = await db.select().from(campaigns).where(eq(campaigns.id, id));
  if (!campaign) notFound();

  return (
    <main className="flex-1 px-6 py-8 max-w-3xl mx-auto w-full">
      <Button asChild variant="ghost" size="sm" className="mb-2 -ml-2">
        <Link href={`/campaigns/${id}`}>← {campaign.name}</Link>
      </Button>
      <h1 className="text-3xl font-bold mb-1">Paramètres : {campaign.name}</h1>
      <p className="text-muted-foreground mb-8">
        Identité, style visuel des images et réglages de l’IA.
      </p>

      <CampaignSettingsForm
        campaignId={campaign.id}
        initial={{
          name: campaign.name,
          description: campaign.description ?? "",
          styleGuide: (campaign.styleGuide ?? {}) as StyleGuide,
        }}
      />
      <div className="mt-6">
        <AiSettingsCard
          campaignId={campaign.id}
          initial={campaign.aiSettings}
          defaultModel={defaultModel()}
          defaultImageModel={defaultImageModel()}
          defaultVideoModel={defaultVideoModel()}
        />
      </div>
    </main>
  );
}
