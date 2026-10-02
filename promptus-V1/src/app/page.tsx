import Link from "next/link";
import { Button } from "@/components/ui/button";
import { Card, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";

export default function HomePage() {
  return (
    <main className="flex-1 flex flex-col items-center justify-center px-6 py-16">
      <div className="max-w-3xl text-center space-y-6">
        <h1 className="text-5xl font-bold tracking-tight">Promptus</h1>
        <p className="text-xl text-muted-foreground italic">
          La scène est prête. Que la partie commence.
        </p>
        <p className="text-base text-muted-foreground max-w-2xl mx-auto">
          Un moteur de jeu narratif pour le jeu de rôle — votre régisseur pour la
          préparation, l’ambiance, la résolution des combats et la mémoire de campagne.
        </p>
      </div>

      <div className="mt-12 grid gap-4 md:grid-cols-3 max-w-5xl w-full">
        <Card>
          <CardHeader>
            <CardTitle>Préparation</CardTitle>
            <CardDescription>
              Créer les fiches, importer du YAML, générer les visuels.
            </CardDescription>
          </CardHeader>
        </Card>
        <Card>
          <CardHeader>
            <CardTitle>Partie en direct</CardTitle>
            <CardDescription>
              Poste de pilotage du MJ, résolution automatique des combats, suivi de l’état du monde.
            </CardDescription>
          </CardHeader>
        </Card>
        <Card>
          <CardHeader>
            <CardTitle>Ambiance</CardTitle>
            <CardDescription>
              Sons et images pour porter le récit sans le voler.
            </CardDescription>
          </CardHeader>
        </Card>
      </div>

      <div className="mt-12">
        <Button asChild size="lg">
          <Link href="/campaigns">Entrer dans l’atelier</Link>
        </Button>
      </div>
    </main>
  );
}
