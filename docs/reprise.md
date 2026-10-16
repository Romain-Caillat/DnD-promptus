# Reprise — où on en est (12 octobre 2026)

Note de passage entre deux sessions Claude. À lire après `CLAUDE.md`,
`MEMORY.md` §0 et `docs/bible.md`. **Branche : `docs/game-bible`**
(pas encore fusionnée dans `main`, pas de pull request).

À supprimer une fois la reprise faite : ce qui dure va dans
`MEMORY.md`, `TICKETS.md` et `archive/tickets/`.

## 1. Ce qui est fait

```
 Brainstorm     Promptus « à la Baldur's Gate » : l'Auteur prépare, le jeu raconte,
                le MJ garde un override invisible        → docs/bible.md
 Vision         CLAUDE.md réécrit (règles de design 1, 2, 6 changées)
 Backlog        ancien TICKETS.md gelé → archive/tickets/backlog-avant-bible.md
                nouveau TICKETS.md : jalon 1 (one-shot Corsaires), jalon 2
 Mémoire        MEMORY.md §0 : ce que la bible remplace, et les décisions de design v2
 Maquettes      docs/design/v2/ (README.md, check.cjs)
   cadre-pc.html     validé : habillage B · Octopath, groupe à gauche, ZQSD
   exploration.html  validé : noms connus par joueur, Perception annoncée, une vitesse
```

Commits de la branche, du plus ancien au plus récent : bible → CLAUDE.md →
backlog → cadre → cadre tranché → exploration → exploration tranchée →
cette note.

## 2. Ce qui est commencé

Rien : `design/draw-dialogue-screen` a été validé le 16 octobre 2026
(décisions dans `MEMORY.md` §0 et `archive/tickets/design.md`, nouveau
ticket `hd2d/roll-dice-in-3d`).

## 3. Ce qui reste à faire

### Fait : PixelLab dans les planches (12 octobre 2026)

9 personnages animés (repos + marche, 4 directions), 7 bustes dont
Mère Goulven en 3 expressions, la taverne en 8 images ; 178
générations. Rangés dans `docs/design/v2/assets/` (voir son
`README.md`), branchés dans `cadre-pc.html` et `exploration.html`.

### Fait : la HD-2D vérifiée (12 octobre 2026)

`docs/design/v2/taverne-hd2d.html` : l'image PixelLab de la taverne
rendue jouable (recette dans `MEMORY.md` §0, ticket
`hd2d/build-level-from-key-art`). Romain : « avec la taverne, on est
bon ». La génération est arrêtée ; il reste environ 1 400 générations
jusqu'au 11 novembre.

### Ensuite, phase 1 du jalon 1 (`TICKETS.md`)

```
 design/draw-combat-planes      la bagarre du quai en 3 plans (repartir de docs/design/hd2d/quai.html)
 design/draw-journal-and-sheet  Journal · Inventaire · Fiche
 design/draw-lobby-and-creator  lobby (une classe par joueur) + création
 design/draw-gm-panel           le panneau MJ (Tab), overrides invisibles
 design/write-frame-tokens      porter l'habillage Octopath dans front/src/styles/tokens.css
 engine/rewrite-corsaires-rules les règles des Corsaires au plus près de la 5e, sans sa complexité
 engine/fix-stats-by-class      stats fixées par la classe
```

Puis les phases 2 à 5 (le monde, parler, combattre, autour du jeu) et
la soirée du jalon 1.

### Plus tard

- Fusionner `docs/game-bible` dans `main` (pull request si Romain la demande).
- Décider le sort des planches du canevas (`docs/design/README.md`) une
  fois les planches v2 complètes.
- Hors champ de la bible : le son, le jeu sur téléphone.

## 4. Façon de travailler retenue avec Romain

- En français, bref, avec des schémas ASCII ; un sujet à la fois, et
  deux ou trois questions au plus à la fin.
- Chaque maquette se termine par « À trancher » : les réponses vont
  dans la planche (« Décidé »), dans `MEMORY.md` §0 et dans les tickets
  touchés ; le ticket de maquette est archivé dans
  `archive/tickets/design.md`.
- Quand Romain laisse le choix, choisir, et noter le choix comme tel.
- Contenu réel des Corsaires uniquement : rien d'inventé côté histoire.
- Vérifier chaque planche dans Chromium avant de la livrer.
