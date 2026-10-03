# Brief design — Promptus

Point de départ de l'épic `design` (voir `TICKETS.md`). À lire en
entier avant d'ouvrir une session de design.

## Le produit en une phrase

Un MJ prépare une campagne générée par IA puis la mène **à distance**
avec ses joueurs, chacun sur son téléphone, avec un co-MJ IA qui
propose et un MJ qui valide.

## Qui utilise quoi

| Rôle | Appareil | Moment | Besoin principal |
| --- | --- | --- | --- |
| MJ | Ordinateur (tablette en direct) | Préparation | Générer, relire et corriger une campagne sans s'y noyer |
| MJ | Ordinateur ou tablette | Direct | Tout piloter d'un seul écran : scène, carte, co-MJ, demandes, combat, musique |
| Joueur | **Téléphone** (ordinateur possible) | Direct | Voir la scène, savoir ce qu'il peut faire, agir en deux touches |
| Joueur | Téléphone | Entre deux sessions | Lire « Précédemment… », sa fiche |

Personnage repère : **Marc, le joueur novice** (`archive/bmad/planning-artifacts/product-brief-promptus.md`).
Chaque écran joueur se juge sur sa première session.

## Principes

1. **L'écran est un outil, pas le jeu.** La conversation entre amis
   reste au centre ; l'interface s'efface.
2. **Le MJ a le dernier mot.** Toute proposition IA est un brouillon
   visible et modifiable avant d'atteindre les joueurs.
3. **Le joueur voit toujours ce qu'il peut faire** : actions dérivées
   des règles, cases atteignables surlignées, portées visibles.
4. **Téléphone d'abord pour le joueur** : navigation basse, panneaux qui
   remontent du bas, cibles tactiles, appui long plutôt que survol, zone
   sûre de l'iPhone (conventions de Devotion, `platform/adapt-mobile-layout`).
5. **Immersion** : l'identité visuelle sert l'ambiance de la campagne
   (le style visuel de la bible peut teinter l'interface).
6. **Français**, clair et sobre.

## Ce qu'il faut dessiner

**Joueur (téléphone)** — rejoindre par lien (pseudo, personnage ou
spectateur) · scène en cours (texte, image, vidéo d'intro) · carte avec
brouillard, déplacement au doigt · fiche · actions et demandes au MJ
(résultat qui revient) · combat : initiative, son tour, attaque ·
musique YouTube (lecteur visible, « activer le son ») · journal public ·
récapitulatif.

**MJ, préparation** — liste des campagnes · génération depuis un pitch
(suivi, coût) · relecture : bible, fronts, graphe de nœuds et indices,
fiches · cartes à trois niveaux · médias et budget · système de règles.

**MJ, direct** — scène courante et sorties · carte complète, pions,
brouillard · co-MJ (demander, éditer, envoyer, appliquer) · demandes
des joueurs (valider, refuser, lancer un test) · combat · musique ·
« montrer aux joueurs » · fin de session et récapitulatifs.

## Contraintes

- Cartes : la grille et les murs sont des données ; l'image n'est qu'un
  fond. Hexagones aux niveaux campagne et région, carrés en combat.
- Le joueur ne voit jamais les notes MJ, les cases cachées, ni le nom
  ou les PV des adversaires non révélés.
- YouTube : lecteur visible obligatoire, démarrage muet.
- Thème clair et sombre.

## Références

- V1 : `archive/promptus-v1-nextjs.zip` (ce qui existait, à ne pas copier tel quel)
- Vision V2 : `archive/bmad/planning-artifacts/vision-v2-decisions.md`
- Devotion : conventions mobiles et menus (`MEMORY.md §6` de Devotion)
