# narrator/voice-prepared-branches · Raconter les branches préparées

**État** : à faire · Jalon 1, phase « Parler »

**Réalise** : [Choisir ta réplique](../../regles/jeu/dialogue/choisir-ta-replique.md) · [Bible, « Les rôles »](../../bible.md)

**Objectif** : C'est le jeu qui raconte, sans jamais sortir de ce que l'Auteur a préparé.

**Périmètre** :

- à chaque nœud, l'IA reçoit l'intention préparée et un contexte court (lieu, PNJ, ce que *ce* joueur sait)
- elle rend **du texte seulement** : choix, jets et conséquences viennent du graphe
- texte filtré : pas de nom inconnu du joueur, pas de secret qu'il ignore
- si l'IA échoue ou tarde, le texte préparé s'affiche tel quel
- chaque appel est compté

**Fini quand** : Une soirée entière se joue sans que l'IA ait inventé de choix, de conséquence ou de secret, et un test de fuite le prouve.

**Départ** : `copilot/draft-narration` (gelé), `ai/route-llm-provider`, `ai/count-ai-calls`
