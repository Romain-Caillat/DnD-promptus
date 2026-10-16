# Écran de dialogue

**État** : tranchée · 16/10/2026

**Résout** : [Choisir ta réplique](../regles/jeu/dialogue/choisir-ta-replique.md) · [Ce que tes stats ouvrent](../regles/jeu/dialogue/ce-que-tes-stats-ouvrent.md) · [Les options payantes](../regles/jeu/dialogue/les-options-payantes.md) · [Rejoindre une conversation](../regles/jeu/dialogue/rejoindre-une-conversation.md)

**Solution** : Portrait du PNJ, réplique du narrateur, puis la liste des répliques que ce joueur peut dire ; le serveur filtre la liste pour chaque joueur.

**Schéma** :

```text
 ┌────────┐  GORVIN, tavernier
 │portrait│  « On ne pose pas de questions, ici. »
 └────────┘  💡 Perspicacité : il ment, il a peur de quelqu'un.
  1. « Je cherche une chambre. »
  2. [Persuasion] « Je saurai être discret. »   DD 12
  3. « Une tournée pour tout le monde. »        (manque 2 po)   ← grisée
```

**Comment** :

- portraits façon Fire Emblem, avec quelques expressions
- une réplique fermée est absente, pas grisée (comme Baldur's Gate 3)
- une réplique trop chère est grisée, avec ce qui manque
- qui a rejoint voit l'écran et peut souffler ses répliques exclusives
- maquette : `docs/design/v2/dialogue.html`

**Tickets** : [`play/talk-to-npcs`](../tickets/play/talk-to-npcs.md)
