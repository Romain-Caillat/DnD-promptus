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

### `design/draw-dialogue-screen` — conçu, pas encore écrit

Fichier prévu : `docs/design/v2/dialogue.html`, même gabarit que
`exploration.html` (habillage Octopath, écran 1920 × 1080 mis à
l'échelle). Le contenu vient de `content/campaigns/corsaires/campagne.yaml`
(scène `sc_perroquet`, PNJ `pnj_goulven`, indices `cl_route_goulven` et
`cl_courants_goulven`).

**Disposition** — Le monde reste derrière, assombri et un peu flou. La
fenêtre G est en bas au centre (x 420 à 1500 sur 1920), et elle ne
couvre jamais D (mon perso, en bas à gauche). Un grand portrait pixel
de Mère Goulven est posé sur le bord haut de la fenêtre, à gauche
(façon Fire Emblem), avec trois expressions : neutre, méfiante, ravie.
La fenêtre contient : le nom, la réplique (Cormorant, qui s'écrit au
fil de l'eau et qu'on peut passer), l'indice de Perspicacité s'il y en
a un, puis « Yann répond : » et les choix numérotés. Si Léa a rejoint,
son petit portrait s'affiche en haut à droite avec « Léa écoute ».

**Réglages de la planche** — Vu par : Yann (mène) / Léa (a rejoint) ;
Le dé : aléatoire / 1 naturel / 20 naturel ; Intention préparée :
masquer / montrer (l'intention du graphe sous chaque réplique, pour
montrer « structure figée, voix vivante ») ; Recommencer.

**Le dialogue (graphe)**

```
 accueil (neutre) « Encore des têtes neuves. Ici on boit, on paie… »
   1. « Une chope, et des nouvelles du port. »      → méfiante « Les nouvelles, ça se paie. »
   2. [Persuasion · CHA +2] DD 10                    → jet du d20 au centre de l'écran
        réussite → ravie : voiles anglaises au sud de Belle-Île + jusant de Kerdonis
                   (2 entrées de journal, chez Yann seulement)
        échec    → « Paie ton ardoise d'abord. »  débloque « Régler l'ardoise · 3 PO »
        1 nat.   → ressort une ardoise de 10 PO au nom du Quartier-maître
                   (option affichée mais grisée : « il vous manque 5 PO »)
        20 nat.  → réussite + « le gamin à la vielle en sait plus que moi »
   3. [Perspicacité] « Qu'est-ce que vous voulez en échange ? »
        n'existe que pour Léa (Perspicacité 14) ; elle peut la SOUFFLER
        → elle apparaît chez Yann, marquée « Léa souffle »
        → « Il y a une ardoise de 3 PO au nom de votre équipage… »  débloque l'ardoise
   4. [Ardoise · 3 PO] (débloquée) → elle parle sans jet (règle de la campagne)
   5. Partir.
```

L'indice de Perspicacité n'est vu que par Léa : « elle se moque de
votre charme ; elle veut être payée ». Le jet affiche son calcul : « d20 13
+ CHA 2 = 15 contre DD 10 · Réussite ». Le dé est blanc (jet de
compétence, `MEMORY.md` §2). Touches : 1 à 5 ou clic ; à la manette, la
croix puis A. Le narrateur respecte la règle des noms : rien d'inconnu
du joueur.

**Questions prévues pour Romain**
1. Les options verrouillées (classe, stat) : cachées comme dans BG3, ou
   grisées avec la raison ?
2. Le d20 : au centre de l'écran, ou dans la fenêtre de dialogue ?
3. Le joueur qui rejoint : peut-il souffler toutes les options, ou
   seulement celles que lui seul débloque ?

## 3. Ce qui reste à faire

### Fait : PixelLab dans les planches (12 octobre 2026)

9 personnages animés (repos + marche, 4 directions), 7 bustes dont
Mère Goulven en 3 expressions, la taverne en 8 images ; 178
générations. Rangés dans `docs/design/v2/assets/` (voir son
`README.md`), branchés dans `cadre-pc.html` et `exploration.html`.

### Ensuite, phase 1 du jalon 1 (`TICKETS.md`)

```
 design/draw-dialogue-screen    conçu (§2 ci-dessus), à écrire
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
