# hd2d/roll-dice-in-3d · Rouler un vrai dé en 3D

**État** : à faire · Jalon 1, phase « Parler »

**Réalise** : [Un vrai dé au milieu de l'écran](../../regles/jeu/jets/un-vrai-de-au-milieu-de-l-ecran.md)

**Objectif** : Chaque jet roule un dé en 3D au milieu de l'écran, vu de tous les joueurs concernés.

**Périmètre** :

- dialogue, attaque, sauvegarde, test en exploration : le même dé
- il tombe, roule, s'arrête sur la face tirée par le serveur ; l'animation ne fait que montrer
- puis le calcul : « d20 13 + CHA 2 = 15 contre DD 10 · Réussite »
- facettes ombrées, couleur du type de jet (MEMORY.md, « Visual direction »)
- hypothèse : il ne remplace pas la banderole de critique

**Fini quand** : À la soirée, un jet de Persuasion chez Mère Goulven et une attaque de la bagarre du quai roulent ce dé, et les joueurs lisent le résultat sans explication.

**Départ** : `ui/roll-faceted-dice` (gelé), le dé plat de `docs/design/v2/dialogue.html`
