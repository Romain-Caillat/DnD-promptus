# hd2d/render-board-in-three · Rendre le plateau dans Three.js

**État** : à faire · Jalon 1, phase « Le monde »

**Réalise** : [Bible, « Direction artistique et contenu »](../../bible.md)

**Objectif** : Sur PC, chaque lieu est une scène 3D lumineuse, à la place du canvas.

**Périmètre** :

- la scène Three.js construite depuis `MapData` remplace le canvas partout
- sols, murs, bâtiments, accessoires, eau
- lumières, brume, ombres, halo, flou de profondeur
- caméra basse en exploration

**Fini quand** : La taverne et le quai de Port-Louis sont rendus depuis leur grille, à 60 images/s sur un PC de joueur, brouillard et éléments cachés respectés.

**Départ** : `front/src/features/hd2d/`, `campaign/watch-prep-in-hd2d`
