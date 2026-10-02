import { notFound } from "next/navigation";
import Link from "next/link";
import { db } from "@/lib/db/client";
import { campaigns } from "@/lib/db/schema";
import { eq } from "drizzle-orm";
import { Button } from "@/components/ui/button";
import { Card, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { StartSessionDialog } from "@/components/sessions/start-session-dialog";
import { SessionsList } from "@/components/sessions/sessions-list";

export const dynamic = "force-dynamic";

interface Props {
  params: Promise<{ id: string }>;
}

export default async function CampaignDashboardPage({ params }: Props) {
  const { id } = await params;
  const [campaign] = await db.select().from(campaigns).where(eq(campaigns.id, id));
  if (!campaign) notFound();

  return (
    <main className="flex-1 px-6 py-8 max-w-6xl mx-auto w-full">
      <Button asChild variant="ghost" size="sm" className="mb-2 -ml-2">
        <Link href="/campaigns">← Campagnes</Link>
      </Button>
      <div className="flex items-start justify-between gap-4 flex-wrap">
        <div>
          <h1 className="text-3xl font-bold">{campaign.name}</h1>
          {campaign.description ? (
            <p className="text-muted-foreground mt-1">{campaign.description}</p>
          ) : null}
        </div>
        <Button asChild variant="outline" size="sm">
          <Link href={`/campaigns/${campaign.id}/settings`}>Paramètres</Link>
        </Button>
      </div>

      <section className="mt-8">
        <div className="flex items-center justify-between mb-4">
          <h2 className="text-xl font-semibold">Sessions</h2>
          <StartSessionDialog campaignId={campaign.id} />
        </div>
        <SessionsList campaignId={campaign.id} />
      </section>

      <section className="mt-8">
        <h2 className="text-xl font-semibold mb-4">Monde et ressources</h2>
        <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
          <Link href={`/campaigns/${campaign.id}/entities`}>
            <Card className="hover:border-primary transition-colors h-full">
              <CardHeader>
                <CardTitle className="text-base">Fiches</CardTitle>
                <CardDescription>
                  Sorts, objets, PNJ, monstres, lieux et événements.
                </CardDescription>
              </CardHeader>
            </Card>
          </Link>
          <Link href={`/campaigns/${campaign.id}/rules`}>
            <Card className="hover:border-primary transition-colors h-full">
              <CardHeader>
                <CardTitle className="text-base">Règles</CardTitle>
                <CardDescription>
                  Système de jeu : jets, états, déplacement, actions des joueurs.
                </CardDescription>
              </CardHeader>
            </Card>
          </Link>
          <Link href={`/campaigns/${campaign.id}/audio`}>
            <Card className="hover:border-primary transition-colors h-full">
              <CardHeader>
                <CardTitle className="text-base">Bibliothèque audio</CardTitle>
                <CardDescription>
                  Ambiances, musiques et bruitages disponibles en partie.
                </CardDescription>
              </CardHeader>
            </Card>
          </Link>
        </div>
      </section>
    </main>
  );
}
