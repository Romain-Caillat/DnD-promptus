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
| Épics de portage | À découper une fois le design validé (moteur, campagne, génération, médias, cartes, session, joueur, co-MJ, combat, continuité) |

---

## Épic `design`

Canevas de travail (privé) : https://claude.ai/artifact/AP3z1S5hbhqEiPzi5agTdy —
fondations, construction d'histoire, écran MJ, TV, téléphone joueur.
Décisions retenues : `MEMORY.md` §2.

On repart d'une feuille blanche côté interface. Le V1 a montré que des
écrans empilés au fil des épics ne font pas un jeu : on dessine d'abord
les parcours, puis on code. Point de départ : `docs/design-brief.md`.

### `design/map-user-journeys` · todo

**Pourquoi** — Savoir ce que chacun fait, dans quel ordre, avant de
dessiner un écran.

**Périmètre** — Parcours du MJ (créer une campagne, la générer, la
relire, préparer les médias, lancer la session, la mener, la clore) et
du joueur (rejoindre, choisir un personnage, jouer un tour, demander
une action, lire le récapitulatif).

**Fini quand** — Les parcours sont écrits et validés par Romain.

**Origine** — `docs/design-brief.md`

### `design/build-design-system` · doing

**Pourquoi** — Une identité visuelle propre au jeu (ambiance, typo,
couleurs, iconographie), cohérente entre téléphone et ordinateur.

**Périmètre** — Tokens (couleurs clair/sombre, typo, espacements),
composants de base adaptés de shadcn, ton des textes.

**Fini quand** — Le design system est publié et validé.

**Origine** — `docs/design-brief.md`

### `design/draw-player-screens` · doing

**Pourquoi** — Le joueur joue sur téléphone : c'est l'écran qui décide
si la partie est agréable.

**Périmètre** — Rejoindre, scène en cours, carte (déplacement au
doigt), fiche, actions, combat à son tour, musique, journal.

**Fini quand** — Maquettes téléphone validées ; aucune action courante
ne demande de viser ni de zoomer.

**Origine** — `docs/design-brief.md`

### `design/draw-gm-screens` · doing

**Pourquoi** — Le MJ mène toute la partie depuis un seul écran sans se
perdre.

**Périmètre** — Préparation (campagne, graphe d'histoire, fiches,
cartes, médias, règles, budget) et direct (scène, carte complète,
co-MJ, demandes des joueurs, combat, musique, fin de session).

**Fini quand** — Maquettes ordinateur et tablette validées.

**Origine** — `docs/design-brief.md`

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
