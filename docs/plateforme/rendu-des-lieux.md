# Rendu des lieux

## Une illustration découpée en plans, dans Three.js
> **tranchée** 12/10/2026 · résout : [Un lieu est une illustration peinte](../regles/jeu/apparence.md)

```
 1 illustration ──▶ 2 décor vide ──▶ 3 plans découpés ──▶ scène Three.js
   (l'Auteur choisit)                 (profondeur = ligne de sol)
 4 persos dans le style ─┐
 5 sol jouable ──────────┼──▶ le serveur arbitre déplacement et portée
 6 vie : lueurs, poussière, fumée, flou de profondeur
```

- **Illustration** : `create_image_pro` en 688 × 384 (16:9), avec l'art et le texte lu de la scène, chaque personnage placé ; 25 générations par essai.
- **Décor vide** : même appel, l'illustration choisie en référence et en style, « même salle, sans personne » (25).
- **Plans découpés** : polygones en pixels de l'image autour de ce qui est devant (comptoir, tables), chacun avec sa ligne de sol. Chaque plan est reculé le long de la ligne de vue : l'image est exacte au repos et les plans glissent quand la caméra suit le joueur.
- **Personnages** : alignés avec des extraits de l'illustration en référence ; joueurs tournés dans PixelLab v3 et animés en 4 directions, PNJ avec une boucle de repos de face.
- **Sol jouable** : zone de marche, rectangles bloquants, points d'interaction, en pixels de l'image. C'est la référence du serveur, au même titre que la grille.

**Pourquoi** : l'illustration de la taverne rendue jouable a été validée par Romain (maquette `docs/design/v2/taverne-hd2d.html`, coûts dans `docs/design/v2/assets/README.md`).
**↳ jeu** : l'illustration fait face au lieu, sinon la grille de combat se déforme ; le dire dans le prompt (« vu de face, le fond parallèle à l'image »). Comment la grille de combat se pose sur le décor : ticket `design/draw-combat-planes`.
