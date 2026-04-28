import Link from "next/link";
import { Button } from "@/components/ui/button";
import { Card, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";

export default function HomePage() {
  return (
    <main className="flex-1 flex flex-col items-center justify-center px-6 py-16">
      <div className="max-w-3xl text-center space-y-6">
        <h1 className="text-5xl font-bold tracking-tight">Promptus</h1>
        <p className="text-xl text-muted-foreground italic">
          The stage is set. Let the session begin.
        </p>
        <p className="text-base text-muted-foreground max-w-2xl mx-auto">
          A narrative game engine for tabletop RPGs — your stage director for
          preparation, atmosphere, combat resolution and shared memory.
        </p>
      </div>

      <div className="mt-12 grid gap-4 md:grid-cols-3 max-w-5xl w-full">
        <Card>
          <CardHeader>
            <CardTitle>Preparation</CardTitle>
            <CardDescription>
              Declare entities, import YAML, generate visuals.
            </CardDescription>
          </CardHeader>
        </Card>
        <Card>
          <CardHeader>
            <CardTitle>Live session</CardTitle>
            <CardDescription>
              Cockpit by phase, automatic combat resolution, world state tracking.
            </CardDescription>
          </CardHeader>
        </Card>
        <Card>
          <CardHeader>
            <CardTitle>Atmosphere</CardTitle>
            <CardDescription>
              Sound and image cues to amplify the narrative without stealing it.
            </CardDescription>
          </CardHeader>
        </Card>
      </div>

      <div className="mt-12">
        <Button asChild size="lg">
          <Link href="/campaigns">Enter the studio</Link>
        </Button>
      </div>
    </main>
  );
}
