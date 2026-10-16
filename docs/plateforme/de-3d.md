# Dé 3D au milieu de l'écran

**État** : tranchée · 16/10/2026

**Résout** : [Faire un jet](../regles/jeu/jets/faire-un-jet.md)

**Solution** : Chaque jet fait rouler un dé en 3D au milieu de l'écran de tous les joueurs présents ; il s'arrête sur la face tirée par le serveur, puis le calcul s'affiche.

**Schéma** :

```text
 jet ──▶ ┌──────────┐ ──▶ « d20 13 + CHA 2 = 15 contre DD 12 · Réussite »
         │  dé 3D   │     vu par tous les joueurs présents
         └──────────┘     face tirée par le serveur
```

**Comment** :

- le même dé en dialogue, en combat et en exploration
- facettes ombrées, couleur du type de jet
- il ne remplace pas la banderole de critique

**Pourquoi** : le jet est un moment partagé, comme autour d'une table.

**Tickets** : [`hd2d/roll-dice-in-3d`](../tickets/hd2d/roll-dice-in-3d.md)
