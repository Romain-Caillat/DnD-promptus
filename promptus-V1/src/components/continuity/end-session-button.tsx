"use client";

import { useState } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { Flag, RotateCcw } from "lucide-react";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
  AlertDialogTrigger,
} from "@/components/ui/alert-dialog";
import { Button } from "@/components/ui/button";

/** Terminer (ou rouvrir) la session. */
export function EndSessionButton({ sessionId, ended }: { sessionId: string; ended: boolean }) {
  const queryClient = useQueryClient();
  const [busy, setBusy] = useState(false);

  async function send(reopen: boolean) {
    setBusy(true);
    try {
      const res = await fetch(`/api/sessions/${sessionId}/end`, {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({ reopen }),
      });
      const json = await res.json();
      if (!res.ok) throw new Error(json.error?.message ?? "Action impossible");
      for (const k of ["session", "session-story", "timeline"]) void queryClient.invalidateQueries({ queryKey: [k, sessionId] });
      toast.success(reopen ? "Session rouverte" : "Session terminée : relisez le récapitulatif");
    } catch (e) {
      toast.error(e instanceof Error ? e.message : "Action impossible");
    } finally {
      setBusy(false);
    }
  }

  if (ended) {
    return (
      <Button size="sm" variant="outline" disabled={busy} onClick={() => send(true)}>
        <RotateCcw /> Rouvrir
      </Button>
    );
  }
  return (
    <AlertDialog>
      <AlertDialogTrigger asChild>
        <Button size="sm" variant="outline" disabled={busy}>
          <Flag /> Terminer la session
        </Button>
      </AlertDialogTrigger>
      <AlertDialogContent>
        <AlertDialogHeader>
          <AlertDialogTitle>Terminer la session ?</AlertDialogTitle>
          <AlertDialogDescription>
            Le combat et la musique s’arrêtent, les joueurs ne peuvent plus agir. Un récapitulatif est préparé (factuel, puis
            réécrit par l’IA si elle est configurée) : vous le relisez avant de le publier aux joueurs.
          </AlertDialogDescription>
        </AlertDialogHeader>
        <AlertDialogFooter>
          <AlertDialogCancel>Annuler</AlertDialogCancel>
          <AlertDialogAction onClick={() => send(false)}>Terminer</AlertDialogAction>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  );
}
