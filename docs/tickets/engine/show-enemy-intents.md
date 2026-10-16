# engine/show-enemy-intents · Montrer les intentions ennemies

**État** : à faire · Jalon 1, phase « Combattre »

**Réalise** : [Lire les intentions ennemies](../../regles/jeu/combat/lire-les-intentions-ennemies.md)

**Objectif** : Un joueur perspicace voit ce que l'ennemi va faire avant son tour.

**Périmètre** :

- au début du tour d'un ennemi, le serveur choisit sa cible et son action à l'avance
- il ne les envoie qu'aux joueurs dont la Perspicacité atteint le seuil

**Fini quand** : Un joueur perspicace voit la cible du gobelin avant son tour, un autre non.

**Départ** : `copilot/propose-adversary-turns` (gelé)
