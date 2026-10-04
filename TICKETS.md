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

Le design est terminé (archivé dans `archive/tickets/design.md`) : les
planches du canevas sont la spécification visuelle, `docs/user-journeys.md`
dit quelle planche dessine quel écran. Ce qui reste se vérifie sur la
vraie plateforme, en jouant, et s'itère.

| Épic | Ce que ça apporte |
| --- | --- |
| `platform` | Le squelette (Rust/React/Tauri), la CI, le compte MJ, le déploiement |
| `ui` | Le design system du canevas en composants React : tokens, cartes-boutons, gemmes, cœurs, dés |
| `engine` | Le moteur de règles dans `shared/` : système de règles en données, jets, actions, états, combat, niveaux |
| `campaign` | La campagne : graphe d'histoire, fiches, préparation et relecture par le MJ |
| `ai` | Le fournisseur LLM, le compteur de coût, la génération d'une campagne depuis un pitch |
| `media` | Images, vidéos et musique YouTube des scènes |
| `session` | Inviter, rejoindre, temps réel, projection joueur, salon, scènes, fin de session |
| `player` | L'écran du joueur, téléphone d'abord, puis ordinateur |
| `gm` | L'écran du MJ en direct, ordinateur puis tablette |
| `copilot` | Le co-MJ : brouillons de narration, PNJ, tours des adversaires, vérification des fiches |
| `tv` | L'écran partagé : TV du salon ou fenêtre partagée sur Discord |
| `characters` | Chaque joueur crée son personnage pixel et le voit marcher dans les 4 directions |
| `maps` | Des cartes à trois échelles, dessinées automatiquement en vue 3/4, dans tous les univers |

## Ordre de construction

La méthode derrière cet ordre — quatre chaînes (règles, campagne,
soirée, retour) tirées des deux parties que Romain a menées à la main —
est dans `docs/lecons-des-parties.md`.

**Deux mondes témoins.** Les Corsaires de la Couronne et le Brasier
sont construits en même temps, du premier ticket au dernier : un ticket
n'est fini que lorsqu'il marche sur les deux. Ils sont **réécrits**
dans le format de Promptus à partir de `dnd-save/`, pas convertis, et
tous leurs visuels sont refaits en pixel art. Leurs règles sont des
brouillons que Romain fera évoluer : rien n'est figé, tout s'édite et
se versionne (`MEMORY.md` §6).

Six phases en trois jalons. Chaque jalon se termine par **une vraie
soirée jouée** avec la table de Romain (règle 4) ; chaque phase par une
démo sur les deux mondes. Un ticket ne laisse aucun bouton vers une
fonction d'une phase suivante.

### Jalon 1 · Une vraie soirée

Romain mène à distance, avec six joueurs, l'un des deux mondes ; l'autre
est jouable de bout en bout en test. Le jalon vise ce qui a manqué aux
Corsaires : des règles claires et appliquées pareil, la continuité de
l'histoire, ce que la table sait, un co-MJ pour improviser, et six
joueurs qui ont chacun leur moment.

**Fini quand** — Une session de deux heures est jouée à distance avec six
joueurs, de la création des personnages au dernier combat, sans que
Romain ouvre autre chose que Promptus et Discord ; les réponses des
joueurs en fin de soirée (`session/collect-player-feedback`) disent que
les règles étaient claires, que chacun a eu son moment, et que chacun
sait quoi faire ensuite. Le monde joué n'est pas encore choisi.

**Phase 0 · Le socle** — `platform/scaffold-workspace` ·
`platform/run-ci` · `platform/sign-in-gm` · `platform/deploy-self-hosted` ·
`ui/write-design-tokens`

**Phase 1 · Les deux mondes en données, sans interface** — le moteur,
les deux systèmes de règles en brouillon, les deux mondes réécrits, le
contrôle et la simulation. Démo : un rapport en ligne de commande qui
retrouve les défauts connus des deux systèmes et simule leurs combats,
pour que Romain fasse évoluer ses règles avant qu'une table ne les voie.
`engine/model-rule-system` · `engine/lint-rule-system` ·
`engine/roll-checks` · `engine/resolve-actions` · `engine/apply-conditions` ·
`engine/run-combat` · `engine/simulate-fights` · `maps/model-grid-maps` ·
`campaign/model-story-graph` · `campaign/rewrite-two-worlds`

**Phase 2 · Les fiches en direct** — le premier morceau utilisable à
une vraie table : rejoindre, créer son personnage, la fiche tenue par le
serveur, les actions rapides du MJ, les règles lisibles. Démo : une
table Corsaires et une table Brasier ouvertes côte à côte, six fiches
chacune. `session/stream-live-changes` · `session/project-player-view` ·
`session/invite-and-join` · `session/validate-characters` ·
`campaign/list-campaigns` · `characters/render-layered-sprite` ·
`characters/build-character-creator` · `ui/build-game-components` ·
`player/read-sheet-and-journal` · `player/read-the-rules` ·
`gm/adjust-sheets-fast`

**Phase 3 · La soirée** — scènes, demandes, dés, carte et combat sur
grille (le quai de Port-Louis, une coursive du Cure-Dent), mémoire de la
table, équilibre entre joueurs, co-MJ, fin de session et retour des
joueurs. Démo : la soirée du jalon. `ui/roll-faceted-dice` ·
`session/open-lobby` · `session/drive-scenes` ·
`session/track-table-knowledge` · `session/end-session` ·
`session/collect-player-feedback` · `media/play-youtube-music` ·
`media/draw-pixel-art-assets` · `player/play-scene` · `player/explore-map` ·
`player/fight-turn` · `player/receive-rewards` · `gm/run-live-screen` ·
`gm/run-combat` · `gm/balance-spotlight` · `ai/route-llm-provider` ·
`ai/count-ai-calls` · `copilot/draft-narration` ·
`copilot/propose-adversary-turns` · `maps/render-three-quarter-tiles` ·
`maps/reveal-fog-and-hidden` · `maps/blend-outdoor-terrain` ·
`maps/build-tileset-packs` · `maps/package-theme-packs`

### Jalon 2 · Préparer, et les vaisseaux

**Fini quand** — Romain a écrit dans Promptus l'acte 2 des Corsaires et
l'acte 1 du Brasier, leurs jauges au vert, et les a joués : l'interception
du Greyhound en mer et un combat du Cure-Dent, avec la TV et un
« Précédemment… » entre deux sessions.

**Phase 4 · Préparer** — le modèle de scène complet, la jauge d'un acte
prêt, ce que les joueurs doivent savoir, l'éditeur de règles, la
génération assistée, les images. Démo : les deux actes manquants écrits
dans Promptus. `campaign/edit-rule-system` · `campaign/review-story-graph` ·
`campaign/check-player-knowledge` · `campaign/check-act-readiness` ·
`ai/generate-campaign` · `ai/evaluate-on-real-campaigns` ·
`media/generate-images-and-video` · `maps/edit-map-gm` ·
`maps/generate-map-llm` · `maps/import-image-map` ·
`copilot/check-character-sheets` · `copilot/co-write-backstory`

**Phase 5 · Les vaisseaux, et entre deux sessions** — le combat de
véhicule, commun au brick et au Cure-Dent ; les factions ; la TV ; la
continuité entre sessions. `engine/support-vehicle-combat` ·
`campaign/track-factions-and-goals` · `engine/level-up` ·
`engine/save-against-death` · `session/write-recaps` ·
`session/schedule-sessions` · `session/pair-shared-screen` ·
`tv/show-evening` · `gm/launch-session` · `player/play-between-sessions` ·
`player/face-death` · `player/buy-and-trade`

### Jalon 3 · Les variantes

**Phase 6** — `engine/formalise-house-rules` · `engine/add-srd-preset` ·
`maps/travel-hex-world` · `characters/walk-in-four-directions` ·
`player/play-on-desktop` · `gm/run-on-tablet` · `copilot/listen-by-voice`

Plus tard, sans jalon : `maps/support-hex-combat`.

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

**État** — Squelette en place et vérifié dans PCT 105 : `bun run dev`
lance Postgres 17, le serveur Axum (`:4333`, rechargé par bacon) et
Vite (`:4334`) ; la page d'accueil, en français via `t()`, interroge
`GET /api/health`, qui vérifie la base (503 `DATABASE_UNAVAILABLE`
sinon). `bun run test` passe (3 tests Rust contre la base de test
`promptus_test`, 2 tests Vitest) ; `bun run lint` (fmt, clippy
`-D warnings`, ESLint avec la règle « aucun texte en dur », tsc, knip)
passe ; l'app Tauri compile en debug et en release (`tauri build
--no-bundle`). **Reste** : ouvrir l'app sur le simulateur iOS, ce qui
demande macOS (`tauri ios init` puis `bun run dev:ios` sur le Mac de
Romain). Les icônes Tauri sont provisoires.

### `platform/run-ci` · todo

**Périmètre** — CI GitHub calquée sur Devotion : fmt, clippy, tests
Rust contre Postgres, lint + typecheck + tests front, gitleaks.

**Fini quand** — Chaque PR est vérifiée.

**Origine** — Alignement sur Devotion

### `platform/sign-in-gm` · doing

**Pourquoi** — Le V1 laissait les pages MJ ouvertes à tous.

**Périmètre** — Compte MJ avec passkey (comme Devotion) ; chaque route
MJ exige la session du MJ propriétaire de la campagne ; les joueurs
restent sans compte (jeton haché, `session/invite-and-join`).

**Fini quand** — Un test parcourt chaque route MJ sans session, puis
avec le compte d'un autre MJ : toutes refusent.

**Origine** — `MEMORY.md` §4

**État** — Livré côté serveur et interface, vérifié dans PCT 105. Un
MJ n'a ni mot de passe ni e-mail : un nom affiché et une ou plusieurs
passkeys découvrables (webauthn-rs 0.5, vérification de l'utilisateur
exigée), donc la connexion ne demande aucun identifiant. Session côté
serveur (`gm_sessions`, jeton aléatoire de 256 bits stocké en SHA-256,
30 jours) dans un cookie `promptus_gm` `HttpOnly`, `SameSite=Strict`,
`Path=/api`, `Secure` dès que `PUBLIC_ORIGIN` est en HTTPS. Le premier
compte exige le **code de démarrage** (aléatoire, affiché dans le
journal du serveur au démarrage tant qu'aucun MJ n'existe, ou fixé par
`GM_SETUP_TOKEN`) ; les suivants, une **invitation** à usage unique
(7 jours, hachée, révocable) créée par un MJ connecté. Toutes les routes
MJ (`/api/me`, `/api/auth/sign-out`, `/api/gm-invites`…) passent par le
middleware `require_gm` (401 `UNAUTHENTICATED`) ; une ressource d'un
autre MJ répond 404, comme une ressource inexistante (`owned_by` dans
`back/src/auth/guard.rs`, prêt pour les campagnes). Interface :
`/connexion`, `/inscription` (code prérempli par le lien d'invitation),
accueil MJ avec invitations et déconnexion. Tests : les cérémonies avec
un authentificateur logiciel (inscription, connexion, rejeu, course
entre deux premiers comptes, invitation utilisée, révoquée ou expirée,
passkey d'un autre MJ), le balayage de chaque route MJ sans session,
avec un faux cookie, une session expirée ou fermée, puis avec un autre
MJ. **Reste** : une vraie cérémonie dans un navigateur (Safari iOS,
Chrome) une fois l'app servie en HTTPS (`platform/deploy-self-hosted`),
puis archiver. Hors périmètre, à décider : récupérer un compte dont la
passkey est perdue, ajouter une seconde passkey, limiter le débit des
routes publiques, les passkeys dans l'app Tauri.

### `platform/deploy-self-hosted` · todo

**Pourquoi** — Les joueurs jouent depuis chez eux : l'app doit être
joignable sur Internet, sans VPN.

**Périmètre** — Docker Compose de production (serveur Axum + Postgres
17), migrations au démarrage, sauvegarde avant chaque déploiement,
déploiement continu (modèle `deploy/` de Devotion), `docs/install.md`
et `docs/backup.md`. Sur le serveur de Romain : stack sur PCT 101,
sous-domaine public routé par le Traefik de PCT 100 comme `party`,
enregistrement DNS chez Ionos.

**Fini quand** — Un téléphone en 4G ouvre un lien d'invitation et
rejoint ; une sauvegarde se restaure sur une base vide.

**Origine** — Alignement sur Devotion

---

## Épic `ui`

Le design system dessiné sur le canevas (`MEMORY.md` §2), en composants
React partagés par le téléphone, l'ordinateur, la tablette et la TV.
Chaque composant du canevas (« Composant — … ») a son équivalent ici.

### `ui/write-design-tokens` · todo

**Pourquoi** — Le noir et blanc, les couleurs de stats, les trois
polices et le mouvement doivent être les mêmes partout, sans valeur
recopiée à la main.

**Périmètre** — Tokens Tailwind v4 (`@theme`) : noir et blanc de
l'interface, six couleurs de stats avec leur forme, rouge des dégâts,
matières des six raretés ; Cinzel, Cormorant Garamond, Chakra Petch
servies par l'app (pas de CDN) ; durées et courbes d'animation, avec un
état de repos calme sous `prefers-reduced-motion` (`MEMORY.md` §4). Les
composants shadcn reçoivent ces tokens.

**Fini quand** — Une page de référence dans l'app reproduit la planche
« Fondations » ; animations réduites, rien ne reste figé sur une image
forte.

**Origine** — Planche « Fondations » · ancien ticket `design/build-design-system`

### `ui/build-game-components` · todo

**Pourquoi** — Le jeu tient dans une dizaine de pièces réutilisées
partout ; les refaire écran par écran donnerait dix variantes.

**Périmètre** — Bouton-carte (ivoire pour l'action principale, noir
pour le reste, bascule à l'appui) et grappe arcade du combat ; gemme de
stat en pixels avec sa vague de lumière ; cœurs (2 PV par cœur,
éclatement au coup) ; barre de cases ; horloge de menace ; carte à
jouer (action, indice, scène) avec ses six raretés ; case d'objet ;
badge d'état ; bandeau de statut ; toasts en trois tons ; panneau qui
monte du bas. Chaque composant est une fonction de ses props, sans
appel réseau.

**Fini quand** — Chaque composant du canevas a son équivalent, montré
dans la page de référence avec ses états ; un test par composant qui a
une règle (cœurs au-delà de 20 PV, cases dépensées, rareté lue sans
couleur).

**Origine** — Planches « Jauges et boutons », « Rareté des
compétences », « Objets et butin », « États des personnages »,
« Notifications »

### `ui/roll-faceted-dice` · todo

**Pourquoi** — Le dé est le moment de tension de chaque tour : il doit
rouler, pas afficher un nombre.

**Périmètre** — Dés à facettes d4 à d20, faces ombrées teintées de la
couleur de la stat lancée (blanc pour un simple test) ; les chiffres
roulent puis se posent avec un éclat. Le résultat vient du serveur :
le dé anime un tirage déjà fait, il ne le décide pas.

**Fini quand** — Les sept dés roulent à 60 images/s sur un téléphone
milieu de gamme, et un test prouve que la valeur affichée est celle du
serveur.

**Origine** — Composant « dé » du canevas · `MEMORY.md` §2

---

## Épic `engine`

Le moteur de règles, dans `shared/`, pur et sans base de données.
Le serveur est seul juge (`MEMORY.md` §3). Spécification : le code et
les tests unitaires de `src/lib/engine/` du V1
(`archive/promptus-v1-nextjs.zip`), à porter test par test.

### `engine/model-rule-system` · todo

**Pourquoi** — Les règles sont une donnée de la campagne, pas du code
(`MEMORY.md` §1) : tout le reste en dépend.

**Périmètre** — Le système de règles : caractéristiques, statistiques
de jeu et leur formule, formule de jet, difficultés nommées, actions
(qui deviennent les cartes des joueurs), classes, peuples, états,
ressources, mouvement par échelle de carte, limites de création,
économie d'actions du tour, recharge des actions en tours, actions
débloquées par niveau, progression (par niveau ou par réussite), ce qui
arrive à 0 PV. Rien n'est codé en dur : un système est un fichier de
données (YAML dans `content/rules/`) que Romain édite, et chaque
modification crée une version ; une campagne pointe sur une version.
Deux premiers systèmes, en brouillon : **Corsaires** et **Brasier**,
tirés de `dnd-save/` (`MEMORY.md` §6 : 2 actions par tour, dégâts fixes,
recharges, +1 XP par réussite, hors combat après 3 tours sans soin).
Ils sont faits pour changer : une règle changée ne demande aucun code.

**Fini quand** — Les deux systèmes se chargent ; un combat de chaque
monde se joue dans les tests du moteur ; changer une valeur ou une
règle dans le fichier (recharge, dégâts, actions par tour) change le
résultat sans toucher au code ; les actions offertes à un personnage se
dérivent de sa classe et de son niveau (tests `ruleset.test.ts` portés).

**Origine** — V1 `ruleset.ts`, `ruleset-schema.ts` · planche « Règles »
(moments 1 à 4 et 6) · `dnd-save/DnD-16-05-2026/regles_*.md`

### `engine/lint-rule-system` · todo

**Pourquoi** — Les règles des Corsaires se contredisaient sans que
personne le voie (`docs/lecons-des-parties.md` §2).

**Périmètre** — Contrôles d'un système de règles, sans jamais bloquer :
cohérence (chaque terme employé est défini, chaque renvoi existe, un
seul modèle de dégâts pour joueurs, PNJ et objets, valeurs des PNJ dans
les tables de référence ou marquées comme exceptions, sens des
recharges explicite) ; équilibre (total des caractéristiques par classe,
dégâts par tour, XP par session et niveau atteint en fin de campagne).

**Fini quand** — Sur le premier brouillon du système des Corsaires, le contrôle
signale les deux modèles de dégâts, « précision » non défini, la stat
principale ambiguë, la CA des gardes hors table et le Canonnier à 63
points ; sur le Brasier, le renvoi à `Combat_Sol.md` qui n'existe pas.

**Origine** — `docs/lecons-des-parties.md` §2 et §3

### `engine/roll-checks` · todo

**Pourquoi** — Chaque demande d'un joueur finit en jet ou en refus
expliqué.

**Périmètre** — Dés et formules, avantage et désavantage, critiques,
test contre une difficulté, jet de groupe (la moitié suffit), tirages
côté serveur avec une source de hasard injectable pour les tests.

**Fini quand** — Tests `dice.test.ts` et `skill-check.test.ts` du V1
portés et verts.

**Origine** — V1 `dice.ts`, `skill-check.ts` · planche « Voyager »
(jet de groupe)

### `engine/resolve-actions` · todo

**Pourquoi** — Une carte jouée doit produire le même effet chez tous,
calculé une seule fois.

**Périmètre** — Le résolveur déclaratif du V1 : 20 effets primitifs,
catalogue d'actions, résolution d'attaque (toucher, dégâts, critique),
bonus déjà calculés pour l'affichage des cartes.

**Fini quand** — Tests `resolver.test.ts` du V1 portés et verts.

**Origine** — V1 `resolver.ts`, `catalog.ts`

### `engine/apply-conditions` · todo

**Périmètre** — Les états définis par le système de règles (étourdi,
apeuré, immobilisé, empoisonné, renversé… ; les 14 du SRD le jour où il
existe), leur durée en tours, leurs effets
sur les jets et le mouvement ; ce qui doit être montré au joueur (badge
et effet pixel).

**Fini quand** — Les états des deux mondes s'appliquent dans les tests
du moteur ; tests `conditions.test.ts` du V1 portés.

**Origine** — V1 `conditions.ts` · planche « États des personnages »

### `engine/run-combat` · todo

**Pourquoi** — Le combat est le moment où les règles se voient le plus.

**Périmètre** — Initiative, ordre du tour, tour d'un combattant
(mouvement, action, fin du tour), attaque avec portée et ligne de vue
lues sur la grille (`maps/model-grid-maps`), mort d'un adversaire, fin
du combat et expérience gagnée.

**Fini quand** — Tests `combat.test.ts` et `grid.test.ts` du V1 portés ;
un combat de la démo se joue entièrement dans les tests du moteur.

**Origine** — V1 `combat.ts`, `grid.ts`

### `engine/level-up` · todo

**Périmètre** — Niveaux par expérience ; points de vie au dé de vie ou à
la moyenne, au choix du joueur ; nouvelles cartes de classe.

**Fini quand** — Borin passe niveau 4 avec ses deux options de PV et sa
nouvelle carte, comme sur la planche.

**Origine** — Planche « Entre deux » (moments 1 et 2)

### `engine/save-against-death` · todo

**Pourquoi** — Une mort de personnage ne doit jamais être un accident
de calcul ; le MJ confirme.

**Périmètre** — Ce qui arrive à 0 PV est une donnée du système : jets
contre la mort (trois réussites, trois échecs, 1 et 20 naturels, soin
qui relève), ou hors combat pour la scène après 3 tours sans soin comme
dans le premier brouillon des Corsaires. La mort, quand le système la prévoit, est proposée
par le moteur et confirmée par le MJ.

**Fini quand** — Les sept moments de la planche « Mourir » se rejouent
dans les tests du moteur, la confirmation du MJ comprise.

**Origine** — Planche « Mourir »

### `engine/formalise-house-rules` · todo · à spécifier

**Pourquoi** — Un MJ écrit sa règle en français ; le serveur doit
pouvoir la juger.

**Périmètre** — Le co-MJ traduit une règle maison en règle formelle
(déclencheur, effet, exceptions, ce que voient les joueurs) avec des
cas de test ; le MJ relit et valide. Format formel à définir sur les
effets primitifs existants.

**Origine** — Planche « Règles » (moment 5)

### `engine/add-srd-preset` · todo · à spécifier

**Pourquoi** — Beaucoup de MJ jouent à D&D 5e ; le SRD 5.1 est la partie
libre de ses règles, publiable sous licence Creative Commons.

**Périmètre** — Le SRD comme troisième système de règles, à côté des
deux mondes ; vérifier que le modèle le porte sans cas particulier.

**Origine** — Romain, 4 octobre 2026 (sorti du jalon 1)

### `engine/simulate-fights` · todo

**Périmètre** — Simuler N combats avec les fiches réelles et les
monstres d'une rencontre, sous une version des règles ; taux de victoire,
durée en tours et en minutes estimées à six joueurs, dégâts par classe,
effet d'une règle changée (comparer deux versions). C'est ce qui aurait
permis de roder le combat de vaisseau du Brasier avant de le jouer.

**Fini quand** — Le rapport de la phase 1 simule la bagarre du quai et
un combat au sol du Brasier, et compare deux versions d'une règle.

**Origine** — Planche « Règles » (moment 7) · `MEMORY.md` §6

### `engine/support-vehicle-combat` · todo

**Pourquoi** — Les deux mondes se battent en vaisseau : le brick des
Corsaires contre le Greyhound à l'acte 2, le Cure-Dent dans le Brasier.
Un seul système, deux habillages.

**Périmètre** — Dans le système de règles, en données : un véhicule est
un personnage partagé avec ses ressources (coque ; boucliers ou voilure ;
énergie ou équipage à répartir), ses postes tenus par les joueurs (barre,
pièces d'artillerie, réparations, vigie ou capteurs, liaison), ses armes
avec leur arc et leur portée. Sur la grille : orientation de la proue,
arcs de tir, angle mort, vent ou gravité comme terrain. Avaries
(incendie, brèche, poste hors service), moral comme seconde voie de
victoire, abordage qui bascule en combat au sol sur la carte du pont.
Un tour reste court à six joueurs : chacun agit à son poste en même
temps, le MJ voit l'ensemble. Rodé par `engine/simulate-fights` avant
d'être joué ; les écrans joueur et MJ et la TV s'y adaptent.

**Fini quand** — L'interception du Greyhound et un combat du Cure-Dent
contre une escouade Vorr se simulent, puis se jouent à six, chacun en
moins de 45 minutes ; un abordage passe au combat sur le pont sans
quitter l'écran.

**Origine** — `dnd-save/DnD_07-06-2026/Combat_Vaisseau.md`,
`Fiche_Cure-Dent.md` · Corsaires acte 2 · Romain, 4 octobre 2026

---

## Épic `campaign`

Le modèle de campagne du V1 (bible, fronts, nœuds, indices, entités,
état vivant), et les écrans de préparation du MJ.

### `campaign/model-story-graph` · todo

**Pourquoi** — L'histoire est un graphe, pas un script (`MEMORY.md` §1) :
la génération, le direct et le co-MJ lisent tous ce modèle.

**Périmètre** — Schéma Postgres et types `shared/` : campagne, bible,
fronts et horloges, nœuds, indices, entités (PNJ, monstres, objets,
lieux), état vivant du monde. Une scène porte ce que Romain prépare
déjà (`MEMORY.md` §6) : lieu, ambiance et musiques, déroulement, jets
prévus (stat, difficulté, conséquence d'un 1 ou d'un 20), PNJ,
points clés du MJ, tactique des adversaires, butin, transition. Un PNJ
porte ce qu'il veut et ce qu'il cache. Validateur (chaque révélation atteinte par
au moins trois indices dans des nœuds différents) ; verrou par
campagne sur chaque écriture du monde (`MEMORY.md` §3).

**Fini quand** — Tests `story-validator.test.ts`, `world.test.ts` et
`concurrency.test.ts` du V1 portés ; deux écritures concurrentes ne
s'écrasent pas.

**Origine** — V1 `story.ts`, `story-validator.ts`, `world.ts`, `lock.ts`,
migrations 0001 à 0003

### `campaign/rewrite-two-worlds` · todo

**Pourquoi** — Les deux mondes témoins doivent exister dans Promptus
dès la phase 1, et pas tels quels : leurs défauts sont ce qu'on veut
corriger (`docs/lecons-des-parties.md`).

**Périmètre** — Réécrire, dans le format de `campaign/model-story-graph`,
à partir de `dnd-save/` : l'acte 1 des Corsaires (scènes complètes avec
ce que les joueurs doivent savoir et une accroche par joueur, PNJ,
boutique, butin, combat du quai, musiques par ambiance) et le Brasier
(monde, quatre factions, quatre composants, Cure-Dent, classes, une
première scène jouable). Aucune image importée : chaque visuel est une
description que `media/draw-pixel-art-assets` dessinera en pixel art.
Import et export YAML, pour que Romain relise et corrige les mondes
comme du texte ; l'import du format V1 sert aux tests portés.

**Fini quand** — `bun run worlds` charge les deux mondes ; le contrôle
des règles et le validateur d'histoire tournent dessus ; test
`import-export.test.ts` porté.

**Origine** — `dnd-save/` · V1 `import-export.test.ts`

### `campaign/list-campaigns` · todo

**Périmètre** — Liste des campagnes du MJ, création d'une campagne vide
(univers, préréglage de règles), réglages, budget IA.

**Fini quand** — Le MJ crée une campagne, la retrouve, la rouvre ; un
autre compte MJ ne la voit pas.

**Origine** — Planche « Préparer » (moments 1 à 3)

### `campaign/review-story-graph` · todo

**Pourquoi** — Le MJ relit et corrige tout ce que l'IA propose avant que
ça existe.

**Périmètre** — Écran de relecture : bible, fronts, graphe de nœuds et
indices, fiches ; atelier avec le co-MJ (diff à accepter ou refuser) ;
alerte de cohérence (règle des trois indices) ; validation de la
campagne qui la rend jouable.

**Fini quand** — Le MJ corrige une campagne générée, accepte un diff du
co-MJ, résout une alerte et la valide.

**Origine** — Planche « Préparer » (moments 6 à 8 et 10)

### `campaign/check-player-knowledge` · todo

**Pourquoi** — Aux Corsaires, il manquait aux joueurs des informations
nécessaires pour l'acte 2.

**Périmètre** — Chaque nœud déclare ce que les joueurs doivent savoir
pour y entrer ; à la relecture, le validateur signale un nœud dont une
information n'est donnée nulle part ; en fin de session, le MJ voit ce
qui manque à la table pour la suite, et peut le glisser dans le
« Précédemment… ».

**Fini quand** — Sur la version de l'acte 1 des Corsaires telle que
Romain l'a jouée, l'outil signale ce qui manquait pour l'acte 2.

**Origine** — Romain, 4 octobre 2026 · `MEMORY.md` §6

### `campaign/check-act-readiness` · todo

**Pourquoi** — L'acte 1 du Brasier n'était pas prêt et rien ne le
disait ; aux Corsaires, aucune scène n'était pensée pour un joueur en
particulier.

**Périmètre** — Une jauge par acte : chaque scène a ses champs
(`campaign/model-story-graph`), chaque information nécessaire a au
moins trois chemins, chaque joueur a au moins une accroche dans l'acte,
chaque rencontre a ses adversaires chiffrés et leur tactique, chaque
combat prévu a été simulé. La jauge dit ce qui manque, le MJ décide.

**Fini quand** — La jauge déclare l'acte 1 du Brasier non prêt et dit
pourquoi ; sur l'acte 1 des Corsaires tel qu'il a été joué, elle
signale que la route du Greyhound n'est que dans une scène facultative.

**Origine** — `docs/lecons-des-parties.md` §3 (chaîne 2)

### `campaign/track-factions-and-goals` · todo · à spécifier

**Périmètre** — Jauges d'affinité par faction, où gagner la faveur des
uns fait baisser celle de leurs rivaux ; objectifs de campagne à cocher
(les quatre composants du Brasier) ; un PNJ permanent joué par le co-MJ
(l'IA de bord LUMEN).

**Origine** — `dnd-save/DnD_07-06-2026/Univers.md`

### `campaign/edit-rule-system` · todo

**Pourquoi** — Les règles des deux mondes vont changer souvent : les
éditer doit être aussi simple que les écrire sur une feuille.

**Périmètre** — L'éditeur du système de règles : statistiques et
formules, difficultés, actions et classes, limites de création ; chaque
changement montre ce qu'il touche (contrôle et simulation relancés) et
crée une version, verrouillée pour la prochaine session ; l'historique
des versions se relit et se compare.

**Fini quand** — Les sept moments de la planche « Règles » sont faisables
dans l'app, la règle maison et la simulation comprises (via
`engine/formalise-house-rules` et `engine/simulate-fights`).

**Origine** — Planche « Règles »

---

## Épic `ai`

### `ai/route-llm-provider` · todo

**Pourquoi** — Changer de modèle ou de fournisseur sans toucher au jeu.

**Périmètre** — Trait de fournisseur (LLM, image, vidéo) avec
OpenRouter comme première implémentation ; gabarits de prompts
versionnés ; sorties validées par schéma ; faux fournisseur
déterministe pour les tests et le développement (comme `fake-llm.ts`).

**Fini quand** — Un appel réel à OpenRouter et le faux fournisseur
passent par le même trait ; une sortie hors schéma est rejetée avec une
erreur lisible.

**Origine** — V1 `llm.ts`, `prompt-template.ts`, `scripts/fake-llm.ts`

### `ai/evaluate-on-real-campaigns` · todo

**Pourquoi** — Un prompt ou un modèle qui change ne doit pas dégrader
en silence ce qui marchait.

**Périmètre** — Un jeu d'évaluation tiré des deux parties : scènes
générées comparées au standard de scène, situations réelles de l'acte 1
des Corsaires (refuser l'offre de Vaubernier, l'incident de Jacquot, le
combat du quai à six contre six) rejouées au co-MJ, réponses notées sur
des critères écrits (cohérence avec ce que la table sait, PNJ fidèle à
sa fiche, aucune règle inventée). Lancé à chaque changement de prompt ou
de modèle, résultat comparé au précédent.

**Fini quand** — Changer de modèle produit un rapport qui dit, cas par
cas, ce qui s'est amélioré ou dégradé.

**Origine** — `docs/lecons-des-parties.md` §4

### `ai/count-ai-calls` · todo

**Pourquoi** — Chaque appel IA coûte (`MEMORY.md` §3).

**Périmètre** — Chaque appel LLM, image ou vidéo enregistré avec son
coût ; budget par campagne ; coût estimé avant chaque lot ; un lot qui
dépasserait le budget est refusé.

**Fini quand** — Un test prouve qu'un lot au-delà du budget n'émet aucun
appel.

**Origine** — `MEMORY.md` §3 · planche « Préparer » (coût estimé)

### `ai/generate-campaign` · todo

**Périmètre** — Pitch → bible, fronts, nœuds, indices, entités, en
tâches de fond suivies en direct ; rien n'est appliqué avant la
relecture du MJ (`campaign/review-story-graph`) ; identifiants inventés
écartés.

**Fini quand** — Un pitch produit une campagne valide (validateur vert)
que le MJ relit et applique ; tests `generation.test.ts` du V1 portés.

**Origine** — V1 `generation/pipeline.ts` · planche « Préparer »
(moments 4 et 5)

---

## Épic `media`

### `media/play-youtube-music` · todo

**Pourquoi** — La musique fait l'ambiance, et YouTube impose un lecteur
visible (`MEMORY.md` §4).

**Périmètre** — Une musique YouTube par scène, choisie par le MJ ;
mini-lecteur visible sur le téléphone, démarrage muet et « activer le
son » ; synchronisation entre joueurs, qui se rattrape après une pub. Plusieurs
musiques par scène, rangées par ambiance (exploration, combat, calme,
épique), comme dans la préparation des Corsaires.

**Fini quand** — Trois téléphones entendent la même musique à la même
minute, et le changement de scène change la musique chez tous.

**Origine** — V1 `youtube.ts` · planche « Jouer » (scène)

### `media/draw-pixel-art-assets` · todo

**Pourquoi** — Tout ce que voient les joueurs est en pixel art
(`MEMORY.md` §2) ; les illustrations des deux mondes sont à refaire dans
ce style.

**Périmètre** — Générer en pixel art, à partir des descriptions des
mondes : illustrations de lieux, portraits de PNJ, scènes d'action,
objets ; une palette et une ambiance par monde (le port de 1718 la nuit,
le Brasier) dans un seul style ; génération par IA (type PixelLab), coût
compté, chaque image relue et validée ou relancée par le MJ.

**Fini quand** — Les scènes et les PNJ de l'acte 1 des Corsaires et de
la première scène du Brasier ont leurs images validées, cohérentes avec
les sprites et les cartes.

**Origine** — Romain, 4 octobre 2026

### `media/generate-images-and-video` · todo

**Périmètre** — Les médias des actes écrits ou générés dans Promptus :
images pixel art par scène (`media/draw-pixel-art-assets`) et vidéo
d'introduction, en tâches de fond, stockées sur disque ; le MJ valide
ou relance chaque média ;
coût compté (`ai/count-ai-calls`). Vérifier d'abord le format de sortie
vidéo d'OpenRouter (`MEMORY.md` §4).

**Fini quand** — Une campagne générée a ses images et vidéos validées,
affichées sur les téléphones et la TV.

**Origine** — V1 `media/` · planche « Préparer » (moment 9)

---

## Épic `session`

Le direct : comment une table se réunit et ce qui circule entre les
écrans. Invariants : projection joueur unique, temps réel sans données
de jeu, jetons hachés (`MEMORY.md` §3).

### `session/stream-live-changes` · todo

**Périmètre** — WebSocket Axum par session : « ceci a changé » (Postgres
NOTIFY relayé) et présence ; les clients relisent par l'API ;
reconnexion qui rattrape l'état.

**Fini quand** — Un téléphone coupé dix secondes revient sur l'état
exact de la table sans recharger la page.

**Origine** — V1 `realtime/`, `notify.ts`

### `session/project-player-view` · todo

**Pourquoi** — C'est l'invariant qui protège les secrets du MJ.

**Périmètre** — Un seul point de projection serveur pour tout ce qu'un
joueur (ou la TV) reçoit : pas de notes MJ, pas de cases non révélées,
pas d'objets cachés, ni nom ni PV d'un adversaire non révélé.

**Fini quand** — Tests `projection.test.ts` du V1 portés, plus un test
qui parcourt chaque route joueur et TV.

**Origine** — V1 `projection.ts` · `MEMORY.md` §3

### `session/invite-and-join` · todo

**Périmètre** — Lien d'invitation de la campagne avec message prêt à
coller sur Discord ; le joueur ouvre le lien dans le navigateur de son
téléphone, choisit un pseudo, puis créer, reprendre ou regarder ; jeton
secret sur l'appareil, empreinte seule côté serveur.

**Fini quand** — Marc rejoint depuis son téléphone sans compte, ferme
le navigateur, revient le lendemain et retrouve son personnage.

**Origine** — Planche « Inviter » (moments 1 et 2)

### `session/validate-characters` · todo

**Périmètre** — Le MJ voit sa table se remplir, relit chaque fiche,
valide ou renvoie avec un mot ; le joueur corrige et renvoie, le MJ ne
relit que la différence ; accroches secrètes tirées des histoires.

**Fini quand** — Les moments 3 à 6 de la planche « Inviter » sont
faisables dans l'app.

**Origine** — Planche « Inviter »

### `session/schedule-sessions` · todo

**Périmètre** — Les joueurs donnent leurs disponibilités, le MJ choisit
la date ; rappel avant la session ; le salon ouvre à l'heure dite.

**Fini quand** — Le rappel arrive sur le téléphone de Marc et le mène
au salon d'un toucher.

**Origine** — Planches « Inviter » (moment 7) et « Entre deux » (moment 6)

### `session/open-lobby` · todo

**Périmètre** — Le salon d'avant-session : présence en direct, test du
son, qui joue à distance ; le MJ lance quand la table est là.

**Fini quand** — Le MJ voit les trois joueurs arriver et lance la
session.

**Origine** — Planches « Mener » et « Lancer »

### `session/drive-scenes` · todo

**Pourquoi** — La boucle principale d'une soirée.

**Périmètre** — Le MJ montre une scène (texte lu, image), révèle un
indice ou une zone ; les joueurs proposent une carte d'action ou
« Autre… » ; la demande arrive au MJ en carte : valider, refuser avec
une raison, ou demander un test (difficulté en un geste) ; le résultat
revient au joueur ; le journal garde ce que le groupe sait.

**Fini quand** — Une scène de la démo se joue de la description au
résultat d'un test, chez le MJ et trois joueurs.

**Origine** — Planches « Jouer » et « Mener » · V1 `session/run-action.ts`

### `session/track-table-knowledge` · todo

**Pourquoi** — Aux Corsaires, tenir la continuité de l'histoire et
appliquer les règles pareil d'un bout à l'autre a été le plus dur.

**Périmètre** — Ce que la table sait : indices révélés, PNJ rencontrés,
promesses et dettes, objets clés, visibles par le MJ et les joueurs
(journal) ; les décisions de règle prises en jeu (« escalader le mât :
DEX 10 »), gardées et reproposées quand la même situation revient ;
ce que la prochaine scène demande et que la table ignore encore,
signalé au MJ.

**Fini quand** — Au milieu d'une soirée, le MJ retrouve en un geste ce
que les joueurs savent d'un PNJ et la difficulté donnée la dernière fois
pour la même action.

**Origine** — Romain, 4 octobre 2026 · `MEMORY.md` §6

### `session/collect-player-feedback` · todo

**Pourquoi** — Les leçons des Corsaires sont arrivées par des reproches
oraux, après coup. Elles doivent arriver à chaque session, par écrit.

**Périmètre** — À la fin de la session, chaque joueur répond en trente
secondes sur son téléphone : les règles étaient-elles claires ? as-tu eu
un moment à toi ? sais-tu ce que ton personnage veut faire la prochaine
fois ? Le MJ voit les réponses à côté de ce que Promptus a mesuré
(temps sans agir par joueur, jets contestés, informations manquantes
pour la suite) et garde une note de ce qu'il change.

**Fini quand** — Après la soirée du jalon 1, Romain lit les six
réponses et les mesures sur un seul écran.

**Origine** — `docs/lecons-des-parties.md` §3 (chaîne 4)

### `session/end-session` · todo

**Périmètre** — « Terminer la session » enregistre l'état ; le MJ écrit
ou colle le récapitulatif ; la chronique s'allonge d'une entrée.

**Fini quand** — La session suivante reprend exactement où la
précédente s'est arrêtée.

**Origine** — Planche « Mener » (fin) · V1 `continuity/`

### `session/write-recaps` · todo

**Périmètre** — Le co-MJ rédige le récapitulatif MJ, le « Précédemment… »
des joueurs et l'entrée de chronique ; le MJ relit et publie.

**Fini quand** — Tests `recap.test.ts` et `continuity.test.ts` du V1
portés ; Marc lit le « Précédemment… » le lendemain.

**Origine** — V1 `continuity/recap.ts` · planches « Mener » et « Entre deux »

### `session/pair-shared-screen` · todo

**Périmètre** — La TV ouvre une page qui affiche un code et un QR ; le
MJ tape le code et la TV rejoint la session avec la projection « tous
les joueurs ». Ou le MJ partage cette page dans une fenêtre sur Discord.
Le MJ choisit ce que la TV peut montrer.

**Fini quand** — Une TV s'appaire en moins de 30 secondes et ne montre
rien de ce que la projection joueur refuse.

**Origine** — Planche « Lancer » (moments 1 à 3)

---

## Épic `player`

L'écran du joueur, d'après la planche « Jouer · la soirée de Marc » :
un bandeau de statut, un sujet, une action principale ; quatre onglets
Jeu, Carte, Perso, Journal.

### `player/play-scene` · todo

**Périmètre** — Onglet Jeu : la scène (lieu, texte lu, image), le
mini-lecteur de musique, les indices reçus en cartes, la main de cartes
d'action et « Autre… », le dé à lancer quand le MJ demande un test, le
résultat.

**Fini quand** — Les moments de scène de la planche « Jouer » sont
faisables dans l'app sur un iPhone et un Android.

**Origine** — Planche « Jouer » · V1 écran joueur

### `player/read-the-rules` · todo

**Pourquoi** — Aux Corsaires, les joueurs ont trouvé les règles peu
claires et pas toujours appliquées pareil.

**Périmètre** — Les règles de la campagne en une page, tirées du
système de règles (rien d'écrit à la main qui puisse diverger) ; chaque
jet montre son calcul (dé, stat, bonus, difficulté quand elle est
connue) et son résultat parmi les quatre (1, échec, réussite, 20) ;
chaque carte dit son coût en actions et sa recharge. Quand les règles
ont changé depuis la dernière session, le joueur voit ce qui change
avant la session suivante, jamais en pleine partie.

**Fini quand** — Un joueur qui n'a pas lu les règles explique, après
son premier combat, pourquoi son attaque a raté.

**Origine** — Règle de design 3 · Romain, 4 octobre 2026

### `player/buy-and-trade` · todo

**Périmètre** — L'or ; une boutique ouverte par le MJ (prix, stock,
objet « sous le comptoir » révélé par un jet ou une discussion) ;
marchander par un jet ; partager le butin.

**Origine** — Marché noir de Kerjean, Corsaires acte 1

### `player/explore-map` · todo

**Périmètre** — Onglet Carte : carte révélée, pion glissé au doigt, cases
atteignables surlignées, trajet validé par le serveur, brouillard,
fantôme quand on est invisible.

**Fini quand** — Marc déplace Borin au doigt sans jamais viser ni
zoomer.

**Origine** — Planche « Jouer » (carte)

### `player/fight-turn` · todo

**Périmètre** — « À toi, Borin ! » ; ordre du tour, carte centrée sur la
cible, cœurs et gemmes, main de cartes en éventail, grappe arcade
(gros bouton, objet, fin du tour), dé coloré, dégâts.

**Fini quand** — Le combat de la démo se joue du premier tour au butin.

**Origine** — Planche « Jouer » (combat)

### `player/read-sheet-and-journal` · todo

**Périmètre** — Onglet Perso (fiche, cartes, sac, équiper) et onglet
Journal (ce que le groupe sait).

**Origine** — Planche « Jouer » (onglets) · planches Joueur

### `player/receive-rewards` · todo

**Périmètre** — Toasts qui tombent et s'empilent, butin, compétence
débloquée, niveau.

**Origine** — Planches « Notifications » et « Objets et butin »

### `player/play-between-sessions` · todo

**Périmètre** — Monter de niveau à la fin de la soirée, lire le récap
et la chronique, consulter sa fiche hors session, donner ses dates.

**Fini quand** — Les six moments de la planche « Entre deux » sont
faisables dans l'app.

**Origine** — Planche « Entre deux »

### `player/face-death` · todo

**Périmètre** — Jets contre la mort sur le téléphone, derniers mots,
puis la suite : regarder, créer un nouveau personnage, ou attendre une
accroche du MJ.

**Origine** — Planche « Mourir »

### `player/play-on-desktop` · todo

**Périmètre** — La même partie dépliée sur un grand écran : scène, main,
carte et combat côte à côte ; raccourcis clavier (chiffres pour les
cartes, espace pour le dé).

**Origine** — Planche « Jouer sur ordinateur »

---

## Épic `gm`

### `gm/run-live-screen` · todo

**Pourquoi** — Le MJ mène toute la soirée d'un seul écran.

**Périmètre** — L'écran MJ en direct sur ordinateur : une action
principale par moment, scène et sorties, carte complète avec ce qui est
caché, demandes des joueurs en cartes, journal qui montre aussi le
caché, la table et ses présences, musique.

**Fini quand** — Romain mène une soirée à six joueurs sans quitter cet
écran (les planches en montrent trois : tout doit tenir à six).

**Origine** — Planche « Mener · la soirée de Marc côté MJ »

### `gm/run-combat` · todo

**Périmètre** — Lancer une rencontre (la carte de combat s'ouvre
partout), initiative, jouer les adversaires, poser et retirer des
états, valider le butin.

**Fini quand** — Un combat à six joueurs contre six adversaires (la
bagarre du quai) se mène côté MJ jusqu'au butin.

**Origine** — Planche « Mener » (combat)

### `gm/balance-spotlight` · todo

**Pourquoi** — Divertir six joueurs à la fois est ce qui épuise le MJ.

**Périmètre** — Pour chaque joueur : depuis quand il n'a rien fait ni
demandé, ses demandes en attente, les accroches de son histoire pas
encore jouées ; le co-MJ propose une occasion de lui donner la main
dans la scène en cours.

**Fini quand** — Sur une soirée de deux heures, le MJ est averti dès
qu'un des six joueurs reste vingt minutes sans moment à lui.

**Origine** — Romain, 4 octobre 2026 · `MEMORY.md` §6

### `gm/adjust-sheets-fast` · todo

**Périmètre** — Depuis l'écran MJ, en un geste : +1 XP, retirer ou
rendre des PV, donner un objet, de l'or ; la fiche du joueur change en
direct, et chaque modification va dans l'historique de la session.

**Origine** — `dnd-save/DnD-16-05-2026/prompt_plateforme_fiches.md`

### `gm/launch-session` · todo

**Périmètre** — Le lancement : salon, TV, « Précédemment… » lu ligne à
ligne, première scène.

**Origine** — Planche « Lancer » (moments 4 et 5)

### `gm/run-on-tablet` · todo

**Périmètre** — L'écran MJ sur tablette : rail de grosses cibles à la
place des onglets, demandes au pouce, carte au doigt (un doigt peint le
brouillard, deux déplacent, pincer zoome), propositions du co-MJ en
trois grosses touches.

**Fini quand** — Romain mène une soirée entière depuis un iPad.

**Origine** — Planche « Tablette »

---

## Épic `copilot`

Tout ce que le co-MJ produit est un brouillon que le MJ modifie ou
valide (`MEMORY.md` §3). Spécification : V1 `copilot/` et ses tests.

### `copilot/draft-narration` · todo

**Pourquoi** — Improviser quand les joueurs sortent du prévu est l'autre
difficulté de Romain aux Corsaires.

**Périmètre** — Décrire, conséquences, « et ensuite ? », faire parler un
PNJ selon sa fiche (ce qu'il veut, ce qu'il cache) ; le co-MJ s'appuie
sur ce que la table sait (`session/track-table-knowledge`) pour rester
dans la continuité ; réponses modifiables avant d'être montrées ;
identifiants inventés écartés.

**Fini quand** — Test `copilot.test.ts` du V1 porté ; aucune réponse du
co-MJ n'atteint un joueur sans geste du MJ.

**Origine** — V1 `copilot.ts` · planche « Mener »

### `copilot/propose-adversary-turns` · todo

**Périmètre** — En combat, le co-MJ propose le tour de chaque adversaire
(cible, action, jet déjà résolu) ; le MJ valide, change ou fait fuir.

**Origine** — Planches « Mener » et « Tablette » (moment 4)

### `copilot/check-character-sheets` · todo

**Périmètre** — Le co-MJ vérifie une fiche envoyée contre les limites
de création et propose un mot au joueur ; le MJ décide.

**Origine** — Planches « Inviter » (moments 3 et 4) et « Créer » (moment 6)

### `copilot/co-write-backstory` · todo

**Périmètre** — Le co-MJ pose des questions au joueur pour écrire son
histoire, sans rien inventer à sa place ; il en tire des accroches
secrètes pour le MJ.

**Origine** — Planche « Créer » (moment 7)

### `copilot/listen-by-voice` · todo · à spécifier

**Périmètre** — Dicter au co-MJ (tablette surtout) ; transcription et
proposition à valider.

**Origine** — Planche « Tablette » (moment 5)

---

## Épic `tv`

### `tv/show-evening` · todo

**Périmètre** — L'écran partagé : un point focal à la fois (histoire, dé,
carte ou butin), lisible d'un canapé ; fil d'une ligne en bas ; grands
moments (dés, coups, butin, niveaux, révélations) ; rien de secret.

**Fini quand** — La TV suit une soirée à six joueurs comme sur la
planche, sans action du MJ autre que l'appairage.

**Origine** — Planches « Écran TV » et « TV · la soirée côté TV »

---

## Épic `characters`

Les personnages sont des sprites pixel art en couleur, façon Terraria /
Starbound (voir `MEMORY.md` §2). Dans le design actuel, ils sont fixes
et vus de profil ; dans l'app, chaque joueur fabrique le sien.

### `characters/render-layered-sprite` · todo

**Pourquoi** — Le personnage est une description (couches et couleurs),
pas une image : la carte, la TV, la fiche et le créateur le dessinent
tous à partir d'elle.

**Périmètre** — Format de description dans `shared/` ; rendu en pixels
des couches (corps, barbe, coiffe, tenue, arme…) avec palettes, contour
calculé sans masquer le visage, ombres ; pièces des deux packs de départ,
humains seulement : marins et corsaires de 1718, équipage spatial ;
effets d'état (`MEMORY.md` §2).

**Fini quand** — Les six personnages types de chaque monde et leurs
adversaires (marins de Gueule-Rouge, chasseurs Vorr) sont dessinés par
l'app à partir de leur description, identiques sur le téléphone, l'écran
MJ et la TV.

**Origine** — `docs/design/avatar.py`, `docs/design/sprite-prototype.py`

### `characters/build-character-creator` · todo

**Pourquoi** — Le personnage est la pièce du joueur sur le plateau :
le créer soi-même, c'est s'y attacher dès la première session.

**Périmètre** — Un configurateur en couches (corps, peau, cheveux,
barbe, tenue, armure, arme, accessoire), chaque couche avec sa palette
de couleurs ; aperçu animé en direct ; un bouton « au hasard ». Le
personnage est stocké comme une description (couches + couleurs), pas
comme une image : l'app le dessine, le contour et les ombres sont
calculés. Le MJ fait de même pour les PNJ et les monstres.

Les étapes de règles suivent le créateur d'apparence : peuple, classe,
caractéristiques (limite dépassée signalée, jamais bloquée), histoire,
envoi au MJ.

**Fini quand** — Un joueur crée son personnage depuis son téléphone en
moins de deux minutes et le retrouve sur la carte, la TV et sa fiche ;
les huit moments de la planche « Créer » sont faisables dans l'app.

**Origine** — Romain, session de design du 3 octobre 2026 · planche
« Créer » · prototypes : `docs/design/avatar.py`,
`docs/design/sprite-prototype.py`

### `characters/walk-in-four-directions` · todo

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

### `maps/model-grid-maps` · todo

**Pourquoi** — Tout le reste (rendu, règles, brouillard, génération)
lit la même description de carte.

**Périmètre** — Modèle dans `shared/` : une carte a une échelle
(monde en hexagones, lieu en carrés de 5 m, rencontre en carrés de
1,5 m), une grille de cases (terrain, mur, hauteur, porte, eau…), des
décors posés (avec couvert et terrain difficile), des objets cachés,
des lumières, une ambiance, et des calques avec leur visibilité
(tous, MJ, joueurs). Format versionné, stocké en base, porté depuis les
cartes YAML du V1.

**Fini quand** — Les cartes des deux mondes (le quai de Port-Louis, une
coursive du Cure-Dent) et celles de la démo V1 se chargent dans le
nouveau modèle, et le moteur de règles calcule déplacement et ligne de
vue dessus.

**Origine** — Session de design du 3 octobre 2026 · V1 (format YAML des cartes)

### `maps/render-three-quarter-tiles` · todo

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

### `maps/blend-outdoor-terrain` · todo

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

### `maps/build-tileset-packs` · todo

**Pourquoi** — Chaque décor (crypte, forêt, rue, désert, coursive…)
a besoin de son jeu de tuiles et de ses décors, sans dessiner à la main
à chaque campagne.

**Périmètre** — Format d'un jeu de tuiles (16 tuiles par matière,
faces de mur, décors avec leur emprise et leurs règles) ; génération
par IA une fois par décor (type PixelLab), relue et validée par le MJ,
puis réutilisée ; premiers packs : le port de 1718 (quai, taverne, pont
de navire) et le vaisseau spatial (coursive, passerelle, salle des
machines).

**Fini quand** — Le MJ génère un nouveau décor, le valide, et
l'utilise sur une carte sans retouche manuelle.

**Origine** — Session de design du 3 octobre 2026

### `maps/package-theme-packs` · todo

**Pourquoi** — Les deux mondes témoins sont un monde pirate de 1718 et
un monde spatial : le thème est là dès le jalon 1.

**Périmètre** — Un thème regroupe ses jeux de tuiles, ses pièces de
personnage, ses objets, les noms de ses six statistiques (MAG peut
devenir TECH ou PSY), ses ressources propres (munitions, oxygène,
bruit…) et éventuellement sa police de titres. L'interface, la grille
et le moteur de règles ne changent pas. Le système de règles est une
donnée de la campagne, pas du code.

**Fini quand** — Les Corsaires et le Brasier se jouent avec leur propre
thème (tuiles, pièces de personnage, objets, ressources : poudre,
munitions, énergie) dans les composants existants (cases, horloges,
badges).

**Origine** — Romain, session de design du 3 octobre 2026

### `maps/edit-map-gm` · todo

**Pourquoi** — Le MJ garde le dernier mot sur chaque carte.

**Périmètre** — Éditeur dans l'écran MJ : peindre terrains et murs,
poser décors, portes, objets cachés et lumières, régler l'ambiance,
pinceau de brouillard, révéler un calque ou un objet en direct.

**Fini quand** — Le MJ crée une carte de rencontre complète en moins
de dix minutes et révèle un passage secret pendant la session ; les sept
moments de la planche « Cartes · l'éditeur » sont faisables dans l'app.

**Origine** — Planches « Cartes · trois échelles » et « Cartes · l'éditeur »

### `maps/generate-map-llm` · todo

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

### `maps/import-image-map` · todo

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

### `maps/reveal-fog-and-hidden` · todo

**Pourquoi** — Explorer, c'est découvrir : ce qui n'est pas vu ne doit
pas fuiter.

**Périmètre** — Brouillard (motif de petits carrés) révélé par la
ligne de vue ou par le MJ ; objets cachés visibles du seul MJ ; un
joueur invisible absent de la TV mais visible en fantôme sur son
téléphone. Tout est filtré côté serveur avant l'envoi.

**Fini quand** — Un test prouve qu'aucune donnée d'une case non
révélée ou d'un objet caché n'atteint un client joueur.

**Origine** — `MEMORY.md` §3 (projection joueur)

### `maps/travel-hex-world` · todo

**Pourquoi** — Le voyage entre les lieux est une partie du jeu, pas un
écran de chargement.

**Périmètre** — Carte du monde en hexagones (≈ 10 km) : le groupe est
un seul pion, avance par portions de journée, révèle les hexagones
traversés et découvre les lieux ; un lieu ouvre sa carte (lieu ou
rencontre). En spatial, la même mécanique sert la carte du système.

Côté table : deux routes proposées, vote des joueurs, portions de
journée lues par le MJ, événement proposé par le co-MJ, jet de groupe,
garde de nuit, arrivée qui ouvre la carte du lieu.

**Fini quand** — Le groupe voyage de Valombre à Morneval sur la carte
du monde puis entre dans l'abbaye sans quitter l'écran de jeu ; les sept
moments de la planche « Voyager » sont faisables dans l'app.

**Origine** — Planches « Cartes · trois échelles » et « Voyager »

### `maps/support-hex-combat` · todo · à spécifier

**Pourquoi** — Certains MJ préfèrent les hexagones aussi en combat.

**Périmètre** — Seconde géométrie pour le moteur de règles
(déplacement, portée, ligne de vue) et le rendu ; à décider après les
premières sessions réelles.

**Origine** — Session de design du 3 octobre 2026 (reporté volontairement)
