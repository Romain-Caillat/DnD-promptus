# Apparence et son

## Un lieu est une illustration peinte
> **tranchée** 12/10/2026 · résolue par : [plateforme/rendu-des-lieux.md](../../plateforme/rendu-des-lieux.md)

```
 ┌──────── fond de la carte ────────┐   ← haut de l'image
 │  plans découpés qui glissent     │
 │  quand la caméra suit le joueur  │
 └──────── devant de la carte ──────┘   ← bas de l'image : la grille s'y pose droite
```

- Un lieu est une illustration peinte, vue de face, rendue jouable ; pas une maquette 3D.
- Les personnages sont dessinés dans le style de l'illustration.
- Lueurs, poussière et fumée font vivre la scène.

**Pourquoi** : les volumes construits faisaient « maquette bon marché » ; l'illustration « vend le rêve ».
**↳ campagne** : l'Auteur choisit l'illustration de chaque lieu · **↳ plateforme** : la recette de rendu.

## Interface façon Octopath
> **tranchée** 12/10/2026 · résolue par : —

```
 ┌─groupe─┐                    ┌─ lieu ─┐
 │ 6 persos│     scène         │        │
 └────────┘                    └────────┘
   panneaux bleu nuit translucides · double filet doré
```

- Panneaux bleu nuit translucides, double filet doré, titres et narration dans des polices à l'ancienne.
- Couleurs et formes des stats, cœurs et cases en pixel art gardés.
- Groupe en colonne à gauche ; déplacement en ZQSD (WASD en option).
- Détail des zones : maquette `docs/design/v2/cadre-pc.html`.

**Pourquoi** : retenu sur la maquette du cadre PC.
**↳ campagne** : — · **↳ plateforme** : ticket `design/write-frame-tokens`.

## Musique choisie, synchronisée
> **tranchée** avant la bible · résolue par : [plateforme/musique.md](../../plateforme/musique.md)

```
 scène ──▶ morceaux YouTube de l'Auteur ──▶ joués en même temps chez tous
```

- La musique est choisie, pas générée.
- Chaque scène a plusieurs morceaux, que le jeu enchaîne et synchronise chez tous les joueurs.

**Pourquoi** : la musique choisie par l'Auteur porte l'ambiance mieux qu'une génération.
**↳ campagne** : morceaux par scène · **↳ plateforme** : lecteur YouTube synchronisé.

## Voix entre joueurs
> **provisoire** 16/10/2026 · résolue par : —

```
 joueurs ◀── Discord (à côté) ──▶ joueurs      le jeu n'a pas de chat vocal
```

- Les joueurs se parlent par un outil externe (Discord…).

**Pourquoi** : un chat vocal intégré est une grosse brique sans gain de jeu.
**↳ campagne** : — · **↳ plateforme** : aucun audio entre joueurs à construire.

## Voix du narrateur
> **à définir** · résolue par : —

```
 réplique ──▶ texte à l'écran   (toujours)
          ──▶ voix de synthèse  (plus tard ?)
```

**Question** : le narrateur et les PNJ parlent-ils à voix haute ?
**Options** : texte seul · voix pour le narrateur · une voix par PNJ.
**Recommandation** : texte d'abord ; prévoir dès maintenant un champ « voix » par PNJ, pour l'ajouter plus tard sans refaire.

**Pourquoi** : la voix coûte cher et change le look ; la décider tôt évite de refaire les PNJ.
**↳ campagne** : l'Auteur choisit une voix par PNJ · **↳ plateforme** : synthèse vocale derrière un fournisseur interchangeable.

## Effets sonores
> **à définir** · résolue par : —

```
 taverne · coups · dés  ──▶ ?
```

**Question** : quels bruits le jeu fait-il, et d'où viennent-ils ?
**Options** : aucun · banque de sons fixe · sons par lieu choisis par l'Auteur.
**Recommandation** : aucune ; à trancher après la première partie de test.

**Pourquoi** : hors champ de la bible ; la musique couvre l'ambiance au jalon 1.
**↳ campagne** : — · **↳ plateforme** : —
