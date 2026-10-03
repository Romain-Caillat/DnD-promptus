# TICKETS — Backlog actif

Tout ce qui reste à construire. Un ticket = une unité livrable qui a
un « fini quand » vérifiable.

**Ce qui appartient ici :** le travail à faire.
**Ce qui n'appartient pas ici :** les décisions durables (→ `MEMORY.md`),
le récit de ce qui a été livré (→ `archive/tickets/`).

## Conventions

- **Épics et tickets sont nommés, jamais numérotés.** Identifiant =
  `epic/verbe-objet`, kebab-case anglais.
- **Statut** : `todo` · `doing` · `blocked` (avec la raison).
  Un ticket fini quitte ce fichier pour `archive/tickets/<epic>.md`.
- **Origine** : d'où vient l'exigence, pour pouvoir y revenir.
- Un ticket marqué **à spécifier** n'est pas implémentable en l'état :
  il faut d'abord écrire ses critères d'acceptation.

## Vue d'ensemble

| Épic | Ce que ça apporte |
| --- | --- |
| `design` | Les écrans MJ et joueur dessinés avant d'écrire du code |
| `platform` | Le squelette (Rust/React/Tauri), la CI, le déploiement, le compte MJ |
| `characters` | Chaque joueur crée son personnage pixel et le voit marcher dans les 4 directions |
| `maps` | Des cartes à trois échelles, dessinées automatiquement en vue 3/4, dans tous les univers |
| Épics de portage | À découper une fois le design validé (moteur, campagne, génération, médias, session, joueur, co-MJ, combat, continuité) |

---

## Épic `design`

Canevas de travail (privé) : https://claude.ai/artifact/AP3z1S5hbhqEiPzi5agTdy —
fondations, construction d'histoire, écran MJ, TV, téléphone joueur.
Décisions retenues : `MEMORY.md` §2. Copie des sources du canevas et
mode d'emploi : `docs/design/README.md`.

On repart d'une feuille blanche côté interface. Le V1 a montré que des
écrans empilés au fil des épics ne font pas un jeu : on dessine d'abord
les parcours, puis on code. Point de départ : `docs/design-brief.md`.

### `design/map-user-journeys` · doing — en attente de validation

**Pourquoi** — Savoir ce que chacun fait, dans quel ordre, avant de
dessiner un écran.

**Périmètre** — Parcours du MJ (créer une campagne, la générer, la
relire, préparer les médias, lancer la session, la mener, la clore) et
du joueur (rejoindre, choisir un personnage, jouer un tour, demander
une action, lire le récapitulatif).

**Fini quand** — Les parcours sont écrits et validés par Romain.

**État** — Écrits dans `docs/user-journeys.md` et sur la planche
« Parcours » du canevas : 11 écrans dessinés, 19 à dessiner, 5
questions ouvertes à trancher par Romain (écran partagé TV ou Discord,
moment de la création des personnages, ce qu'un joueur peut faire entre
deux sessions, mort d'un personnage, spectateurs).

**Origine** — `docs/design-brief.md`

### `design/build-design-system` · doing

**Pourquoi** — Une identité visuelle propre au jeu (ambiance, typo,
couleurs, iconographie), cohérente entre téléphone et ordinateur.

**Périmètre** — Tokens (couleurs clair/sombre, typo, espacements),
composants de base adaptés de shadcn, ton des textes.

**Fini quand** — Le design system est publié et validé.

**État** — Dessiné sur le canevas : gemmes de stats, cœurs, cases,
horloges, boutons-cartes, cartes et raretés, objets, dés colorés,
sprites, états, notifications (`MEMORY.md` §2). Reste : tokens écrits
(couleurs, typo, espacements) dans un format réutilisable par le code,
mode clair, ton des textes.

**Origine** — `docs/design-brief.md`

### `design/draw-player-screens` · doing

**Pourquoi** — Le joueur joue sur téléphone : c'est l'écran qui décide
si la partie est agréable.

**Périmètre** — Rejoindre, scène en cours, carte (déplacement au
doigt), fiche, actions, combat à son tour, musique, journal.

**Fini quand** — Maquettes téléphone validées ; aucune action courante
ne demande de viser ni de zoomer.

**État** — La soirée de Marc (joueur novice) est jouable de bout en bout
sur la planche « Jouer · la soirée de Marc » : scène, indices, dé,
exploration, combat (main de cartes, « Autre… », arcade), butin, onglets
Jeu, Carte, Personnage, Journal. Les planches Joueur et le storyboard
sont ce même téléphone figé sur un moment. Reste à dessiner : rejoindre
(lien, choix), créateur de personnage, « Précédemment… » hors session,
voyage sur la carte du monde, fin de session et récapitulatif, joueur
sur ordinateur, mort du personnage, mini-lecteur YouTube visible
(`MEMORY.md` §4).

**Origine** — `docs/design-brief.md`

### `design/draw-gm-screens` · doing

**Pourquoi** — Le MJ mène toute la partie depuis un seul écran sans se
perdre.

**Périmètre** — Préparation (campagne, graphe d'histoire, fiches,
cartes, médias, règles, budget) et direct (scène, carte complète,
co-MJ, demandes des joueurs, combat, musique, fin de session).

**Fini quand** — Maquettes ordinateur et tablette validées.

**État** — Dessinés : construction d'histoire avec le LLM, écran MJ en
direct (une version, avant la soirée de Marc), cartes (vue MJ, import
d'image). Reste : la soirée de Marc côté MJ (prochaine étape : la même
soirée, moment par moment, comme le téléphone et la TV), liste et
création de campagne, éditeur du système de règles, suivi de génération
et coût, fiche PNJ/monstre, éditeur de carte, médias et budget,
invitation et validation des personnages, salon, jumelage de l'écran
partagé, voyage, fin de session, version tablette.

**Origine** — `docs/design-brief.md`

### `design/verify-canvas-rendering` · doing

**Pourquoi** — Les planches ont été écrites sans pouvoir les voir : le
rendu n'a été contrôlé que par des scripts (classes définies, logique
des 12 moments en Node). Romain a déjà trouvé des cadres déformés.

**Périmètre** — Ouvrir chaque planche du canevas, la comparer à son
intention (`MEMORY.md` §2, `docs/design/README.md`), corriger ce qui
déborde, se chevauche ou ne s'aligne pas. Priorité : « Jouer · la
soirée de Marc » (cliquer tous les moments), « TV · la soirée côté
TV », « Joueur · la soirée de Marc », les planches Joueur, puis les
autres.

**Fini quand** — Chaque planche a été vue rendue, et Romain valide les
trois planches de la soirée.

**État** — Les trois planches de la soirée ont été vues rendues dans
Chrome le 3 octobre 2026 (les 12 moments de « Jouer » cliqués) et
corrigées : personnages cachés sous les hauts de mur (rayons X), brouillard
en carrés blancs quand les animations sont réduites, bannière « À toi,
Borin ! » sous la carte, titre de carte d'indice décentré, main de cartes
hors de l'écran, libellé « Frappe » illisible, sélecteur de moments qui
sautait, dégâts TV au-dessus du mauvais personnage. Toutes les autres
planches ont ensuite été vues rendues et corrigées : horloge « 3/6 » empilée
en colonne, gemme de carte sur le titre, cadres trop courts (Fondations,
Jauges, Objets), butin sur son nom, dé TV sur les PV du groupe, « Tu es
ici » sur Lyra, calibrage sur « passage secret », brouillard des
extérieurs, barres de défilement des téléphones. Reste : validation des
trois planches de la soirée par Romain. Vus, non corrigés : l'écran MJ
déborde même à 1440 px (phases, demandes, carte, dés — à reprendre avec
la soirée côté MJ) ; Histoire laisse un grand vide sous 1440 px ; petits
défauts de Notifications ; barres de défilement des planches Sprite et
Carte à jouer vues seules (cause hors du code des planches).

**Origine** — Romain, 3 octobre 2026 : « TV · la soirée côté TV est
cassé », « c'est pas aligné ».

### `design/scope-legacy-boards` · todo

**Pourquoi** — Le CSS d'une planche s'applique aux composants qu'elle
importe (`MEMORY.md` §4). Seules les planches de la soirée sont
préfixées ; les anciennes (Main, Pistes-UI, Objets, États, Notifs,
Cartes, Extérieurs, Parcours, MJ, Histoire) utilisent des noms de
classes génériques et peuvent casser un composant sans qu'on le voie.

**Périmètre** — Passer chaque ancienne planche dans
`docs/design/scope.py` avec son préfixe, vérifier le rendu avant et
après.

**Fini quand** — Plus aucune planche n'a de classe non préfixée hors
modificateurs d'état, et le rendu est inchangé ou corrigé.

**Origine** — Correctif de la TV du 3 octobre 2026

---

## Épic `platform`

### `platform/scaffold-workspace` · todo

**Pourquoi** — Démarrer la réécriture sur la même base que Devotion.

**Périmètre** — Workspace Cargo (`back/`, `shared/`, `src-tauri/`),
`front/` React + Vite + shadcn + i18n, scripts Bun (`dev`, `test`,
`db:start`…), `docker-compose.dev.yml` (Postgres 17), base de test
séparée (`TEST_DATABASE_URL`), `.env.example`.

**Fini quand** — `bun run dev` lance base, serveur et interface ;
`bun run test` passe ; l'app Tauri s'ouvre sur le simulateur iOS.

**Origine** — Alignement sur Devotion

### `platform/run-ci` · todo

**Périmètre** — CI GitHub calquée sur Devotion : fmt, clippy, tests
Rust contre Postgres, lint + typecheck + tests front, gitleaks.

**Fini quand** — Chaque PR est vérifiée.

**Origine** — Alignement sur Devotion

### `platform/sign-in-gm` · todo · à spécifier

**Pourquoi** — Le V1 laissait les pages MJ ouvertes à tous.

**Périmètre** — Compte MJ avec passkey (comme Devotion) ; les joueurs
restent sans compte (jeton haché).

**Origine** — `MEMORY.md` §4

### `platform/deploy-self-hosted` · todo · à spécifier

**Périmètre** — Docker Compose de production, migrations au démarrage,
sauvegarde avant chaque déploiement, déploiement continu (modèle
`deploy/` de Devotion), `docs/install.md` et `docs/backup.md`.

**Origine** — Alignement sur Devotion

---

## Épic `characters`

Les personnages sont des sprites pixel art en couleur, façon Terraria /
Starbound (voir `MEMORY.md` §2). Dans le design actuel, ils sont fixes
et vus de profil ; dans l'app, chaque joueur fabrique le sien.

### `characters/build-character-creator` · todo · à spécifier

**Pourquoi** — Le personnage est la pièce du joueur sur le plateau :
le créer soi-même, c'est s'y attacher dès la première session.

**Périmètre** — Un configurateur en couches (corps, peau, cheveux,
barbe, tenue, armure, arme, accessoire), chaque couche avec sa palette
de couleurs ; aperçu animé en direct ; un bouton « au hasard ». Le
personnage est stocké comme une description (couches + couleurs), pas
comme une image : l'app le dessine, le contour et les ombres sont
calculés. Le MJ fait de même pour les PNJ et les monstres.

**Fini quand** — Un joueur crée son personnage depuis son téléphone en
moins de deux minutes et le retrouve sur la carte, la TV et sa fiche.

**Origine** — Romain, session de design du 3 octobre 2026 ·
prototype : `docs/design/sprite-prototype.py`

### `characters/walk-in-four-directions` · todo · à spécifier

**Pourquoi** — Sur la carte, un personnage qui se tourne vers là où il
va rend le déplacement lisible et vivant.

**Périmètre** — Chaque couche existe en 4 directions (face, dos,
gauche, droite) ; animations repos, marche, attaque, touché par
direction ; le pion se tourne vers sa case d'arrivée ou sa cible.

**Fini quand** — Un déplacement sur la grille montre le personnage
marcher dans la bonne direction, chez tous les joueurs et sur la TV.

**Origine** — Romain, session de design du 3 octobre 2026

---

## Épic `maps`

Décisions : `MEMORY.md` §2 (trois échelles, vue 3/4, image = décor,
extérieurs, thèmes en packs). Planches « Cartes · trois échelles » et
« Cartes · extérieurs » du canevas. Prototypes de rendu :
`docs/design/walls-prototype.py`, `docs/design/outdoor/`.

Règle commune à tous les tickets : **la grille porte les règles**, le
dessin n'est qu'une projection. Le serveur reste seul juge du
déplacement, de la portée et de la ligne de vue ; un joueur ne reçoit
jamais ce qu'il n'a pas le droit de voir (projection joueur, `MEMORY.md` §3).

### `maps/model-grid-maps` · todo · à spécifier

**Pourquoi** — Tout le reste (rendu, règles, brouillard, génération)
lit la même description de carte.

**Périmètre** — Modèle dans `shared/` : une carte a une échelle
(monde en hexagones, lieu en carrés de 5 m, rencontre en carrés de
1,5 m), une grille de cases (terrain, mur, hauteur, porte, eau…), des
décors posés (avec couvert et terrain difficile), des objets cachés,
des lumières, une ambiance, et des calques avec leur visibilité
(tous, MJ, joueurs). Format versionné, stocké en base, porté depuis les
cartes YAML du V1.

**Fini quand** — Les cartes de la campagne de démo V1 se chargent dans
le nouveau modèle et le moteur de règles calcule déplacement et ligne
de vue dessus.

**Origine** — Session de design du 3 octobre 2026 · V1 (format YAML des cartes)

### `maps/render-three-quarter-tiles` · todo · à spécifier

**Pourquoi** — Le MJ ou le LLM ne pose que des cases ; la carte doit
se dessiner seule, belle, dans le style des personnages.

**Périmètre** — Rendu en vue 3/4 par autotiling (double grille,
16 tuiles par matière) : dessus des murs décalé d'une case, face quand
la case du dessous est ouverte, ombres portées ; reliefs comme des murs
qu'on gravit (falaise en strates, unité en hauteur dessinée plus haut) ;
ordre de dessin par rangée pour passer derrière un mur ; calque de
cimes au-dessus des pions, éclairci côté MJ quand un pion est dessous.
Même rendu sur la TV, le téléphone et l'écran MJ.

**Fini quand** — La crypte, la rue zombie, le désert et la forêt des
planches sont rendus par l'app à partir de leur grille, sans image
préparée, à 60 images/s sur un téléphone milieu de gamme.

**Origine** — Planches « Cartes » du canevas · `docs/design/walls-prototype.py`

### `maps/blend-outdoor-terrain` · todo · à spécifier

**Pourquoi** — Sans extérieurs crédibles, on ne joue ni une rue
post-apo ni une bataille sur une planète.

**Périmètre** — Terrains fondus par bruit (bitume, dalles, herbe,
terre, sable, roche, eau…) ; décors en pixel posés sur la grille
(voitures, barricades, arbres, rochers, caisses) qui portent leurs
règles (couvert ½ ou ¾, terrain difficile, passage obligé) ; ambiance
réglable : heure, météo (pluie, sable, brume), sources de lumière
(feu, cristaux) qui éclairent et vacillent.

**Fini quand** — Le MJ change l'heure ou la météo d'une carte en
direct et les joueurs le voient ; un décor de couvert modifie bien le
jet d'attaque calculé par le serveur.

**Origine** — Planche « Cartes · extérieurs » · `docs/design/outdoor/`

### `maps/build-tileset-packs` · todo · à spécifier

**Pourquoi** — Chaque décor (crypte, forêt, rue, désert, coursive…)
a besoin de son jeu de tuiles et de ses décors, sans dessiner à la main
à chaque campagne.

**Périmètre** — Format d'un jeu de tuiles (16 tuiles par matière,
faces de mur, décors avec leur emprise et leurs règles) ; génération
par IA une fois par décor (type PixelLab), relue et validée par le MJ,
puis réutilisée ; premiers packs : crypte et forêt (fantasy), rue
(zombies), désert et coursive (spatial).

**Fini quand** — Le MJ génère un nouveau décor, le valide, et
l'utilise sur une carte sans retouche manuelle.

**Origine** — Session de design du 3 octobre 2026

### `maps/package-theme-packs` · todo · à spécifier

**Pourquoi** — On jouera aussi en zombies et en spatial, pas seulement
en fantasy.

**Périmètre** — Un thème regroupe ses jeux de tuiles, ses pièces de
personnage, ses objets, les noms de ses six statistiques (MAG peut
devenir TECH ou PSY), ses ressources propres (munitions, oxygène,
bruit…) et éventuellement sa police de titres. L'interface, la grille
et le moteur de règles ne changent pas. Le système de règles est une
donnée de la campagne, pas du code.

**Fini quand** — Une campagne zombie et une campagne spatiale se
jouent de bout en bout avec leurs ressources affichées dans les
composants existants (cases, horloges, badges).

**Origine** — Romain, session de design du 3 octobre 2026

### `maps/edit-map-gm` · todo · à spécifier

**Pourquoi** — Le MJ garde le dernier mot sur chaque carte.

**Périmètre** — Éditeur dans l'écran MJ : peindre terrains et murs,
poser décors, portes, objets cachés et lumières, régler l'ambiance,
pinceau de brouillard, révéler un calque ou un objet en direct.

**Fini quand** — Le MJ crée une carte de rencontre complète en moins
de dix minutes et révèle un passage secret pendant la session.

**Origine** — Planche « Cartes · trois échelles »

### `maps/generate-map-llm` · todo · à spécifier

**Pourquoi** — Préparer une carte doit être aussi rapide que décrire
la scène.

**Périmètre** — À partir d'une scène du graphe d'histoire, le LLM
propose une grille (murs, portes, terrains, décors, objets cachés)
dans le jeu de tuiles du décor ; le MJ la relit dans l'éditeur et la
valide. Rien n'arrive aux joueurs sans validation ; chaque appel est
compté.

**Fini quand** — Une scène de la démo produit une carte jouable,
validée par le MJ, en une génération et quelques retouches.

**Origine** — Règles de design 1 et 2 (`CLAUDE.md`)

### `maps/import-image-map` · todo · à spécifier

**Pourquoi** — Beaucoup de MJ ont déjà des cartes (achetées, faites
dans Dungeondraft ou Dungeon Alchemist, générées).

**Périmètre** — Importer ou générer une image ; aligner la grille
(taille de case, décalage, case témoin) ; tracer les murs, ou laisser
l'IA les proposer puis les valider ; importer le format Universal VTT
(`.dd2vtt`) avec ses murs et lumières ; l'image n'est qu'un décor,
tout ce qui est caché ou change d'état est un objet posé par-dessus.

**Fini quand** — Une carte `.dd2vtt` et une image brute deviennent
jouables, avec ligne de vue correcte, en moins de cinq minutes.

**Origine** — Recherche sur Foundry VTT et Owlbear Rodeo, session du 3 octobre 2026

### `maps/reveal-fog-and-hidden` · todo · à spécifier

**Pourquoi** — Explorer, c'est découvrir : ce qui n'est pas vu ne doit
pas fuiter.

**Périmètre** — Brouillard (motif de petits carrés) révélé par la
ligne de vue ou par le MJ ; objets cachés visibles du seul MJ ; un
joueur invisible absent de la TV mais visible en fantôme sur son
téléphone. Tout est filtré côté serveur avant l'envoi.

**Fini quand** — Un test prouve qu'aucune donnée d'une case non
révélée ou d'un objet caché n'atteint un client joueur.

**Origine** — `MEMORY.md` §3 (projection joueur)

### `maps/travel-hex-world` · todo · à spécifier

**Pourquoi** — Le voyage entre les lieux est une partie du jeu, pas un
écran de chargement.

**Périmètre** — Carte du monde en hexagones (≈ 10 km) : le groupe est
un seul pion, avance par portions de journée, révèle les hexagones
traversés et découvre les lieux ; un lieu ouvre sa carte (lieu ou
rencontre). En spatial, la même mécanique sert la carte du système.

**Fini quand** — Le groupe voyage de Valombre à la crypte sur la carte
du monde puis entre dans la crypte sans quitter l'écran de jeu.

**Origine** — Planche « Cartes · trois échelles »

### `maps/support-hex-combat` · todo · à spécifier

**Pourquoi** — Certains MJ préfèrent les hexagones aussi en combat.

**Périmètre** — Seconde géométrie pour le moteur de règles
(déplacement, portée, ligne de vue) et le rendu ; à décider après les
premières sessions réelles.

**Origine** — Session de design du 3 octobre 2026 (reporté volontairement)
