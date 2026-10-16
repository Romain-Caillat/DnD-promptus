# play/fight-turn · Jouer son tour de combat

**État** : à faire · Jalon 1, phase « Combattre »

**Réalise** : [Écran de combat](../../plateforme/ecran-de-combat.md)

**Objectif** : Chaque joueur joue ses tours sans aide.

**Périmètre** :

- vue tactique : cases accessibles, portées, ressources
- aperçu : % de toucher et dégâts ; annuler avant de valider
- minuteur, initiative en frise

**Fini quand** : Chaque joueur joue ses tours de la bagarre du quai sans aide.

**Départ** : `player/fight-turn` (gelé), `engine/run-combat`
