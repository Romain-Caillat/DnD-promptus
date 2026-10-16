# Rendu des lieux peints

**État** : tranchée · 12/10/2026

**Résout** : [Un lieu est une illustration peinte](../regles/jeu/apparence/un-lieu-est-une-illustration-peinte.md)

**Solution** : l'illustration choisie par l'Auteur devient un décor vide, découpé en plans posés en profondeur dans une scène Three.js ; le sol jouable, en pixels de l'image, est la référence du serveur.

**Schéma** :

```text
 1 illustration ──▶ 2 décor vide ──▶ 3 plans découpés ──▶ scène Three.js
   (l'Auteur choisit)                 (profondeur = ligne de sol)
 4 persos dans le style ─┐
 5 sol jouable ──────────┼──▶ le serveur arbitre déplacement et portée
 6 vie : lueurs, poussière, fumée, flou de profondeur
```

**Comment** :

- Illustration : `create_image_pro` en 688 × 384 (16:9), art et texte lu de la scène, chaque personnage placé ; 25 générations par essai. Prompt : « vu de face, le fond parallèle à l'image ».
- Décor vide : même appel, l'illustration choisie en référence et en style, « même salle, sans personne » (25).
- Plans : polygones autour de ce qui est devant (comptoir, tables), chacun avec sa ligne de sol, reculés le long de la ligne de vue ; l'image est exacte au repos et glisse quand la caméra suit le joueur.
- Personnages : extraits de l'illustration en référence ; joueurs tournés dans PixelLab v3 et animés en 4 directions, PNJ avec une boucle de repos de face.
- Sol jouable : zone de marche, rectangles bloquants, points d'interaction, en pixels de l'image.

**Limites** : une illustration de biais (rue en diagonale) déforme la grille de combat ; comment la grille se pose sur le décor reste à dessiner (`design/draw-combat-planes`). Maquette : `docs/design/v2/taverne-hd2d.html`.

**Tickets** : [`hd2d/build-level-from-key-art`](../tickets/hd2d/build-level-from-key-art.md)
