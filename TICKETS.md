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

*État* : tout est codé et testé (API, sweeps de fuite, écrans MJ et
joueur) ; reste la soirée du jalon jouée pour de vrai, sur de vrais
téléphones, avec un fournisseur d'IA configuré.

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

*État* : les cinq lots sont fusionnés, codés et testés (combat de
véhicule, factions et objectifs, niveaux et mort, récaps, dates et
rappels, TV jumelée, lancement de soirée, entre-deux-séances, marché) ;
reste l'interception du Greyhound et un combat du Cure-Dent joués pour de
vrai, avec une vraie TV, de vrais téléphones et un vrai rappel Discord,
puis les relectures de Romain (affinités, monnaie du Brasier).

### Jalon 3 · Les variantes

**Phase 6** — `engine/formalise-house-rules` · `engine/add-srd-preset` ·
`maps/travel-hex-world` · `characters/walk-in-four-directions` ·
`player/play-on-desktop` · `gm/run-on-tablet` · `copilot/listen-by-voice`

*État* : les sept tickets sont codés et intégrés sur une branche
(fmt, clippy, tsc, eslint, knip et Vitest sous Bun au vert, tests sans
base au vert) ; les tests serveur qui passent par la base n'ont pas tourné
après intégration (Postgres de dev hors service). Restent ces tests, puis
les essais réels : une règle maison et une dictée avec la clé OpenRouter,
une campagne SRD jouée, un voyage et des déplacements vus sur de vrais
téléphones et à la TV, une soirée jouée sur ordinateur et une menée
depuis un iPad.

Plus tard, sans jalon : `maps/support-hex-combat`.

---

## Épic `platform`

### `platform/scaffold-workspace` · doing — reste le simulateur iOS (Mac)

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

### `platform/run-ci` · doing — reste le premier passage sur GitHub

**Périmètre** — CI GitHub calquée sur Devotion : fmt, clippy, tests
Rust contre Postgres, lint + typecheck + tests front, gitleaks.

**Fini quand** — Chaque PR est vérifiée.

**Origine** — Alignement sur Devotion

**État** — `.github/workflows/ci.yml` écrit, sur chaque PR et chaque
push sur `main`, en trois jobs : `rust` (dépendances système de Tauri,
création de `promptus_test` par `scripts/create-test-db.sql` dans un
service Postgres 17, `cargo fmt --all --check`, clippy `--workspace
--all-targets -D warnings`, `cargo test --workspace`), `front` (Bun
1.4.2 comme dans PCT 105 : ESLint, `tsc -b`, knip, Vitest, sous Bun
via `--bun`) et `secrets` (gitleaks 8.30.1, binaire épinglé et vérifié
par somme SHA-256, sur tout l'historique). Le fichier passe
`actionlint` (avec shellcheck), et chaque commande a été rejouée dans
PCT 105 avec les mêmes variables — seule différence, le port 5433 d'un
Postgres 17 jetable à la place du service. **Reste** : le premier
passage sur GitHub (pousser, regarder les trois jobs passer), puis
rendre les trois jobs obligatoires dans la protection de `main`.

### `platform/sign-in-gm` · doing — reste une vraie cérémonie en HTTPS

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

### `platform/deploy-self-hosted` · doing — reste le choix de l'hôte et la mise en ligne

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

**État** — Tout ce qui ne demande pas d'exposition publique est fait et
vérifié dans PCT 105, sur une pile de test séparée
(`promptus-prod-test`, `127.0.0.1:4380`, démontée ensuite) : un
`Dockerfile` en trois étages (front Bun, serveur Rust, image
`debian:trixie-slim` de ~170 Mo) où le serveur Axum sert aussi le front
construit (`FRONT_DIR`, routes du client → `index.html`, `/assets`
en cache long, `/api/*` inconnu → 404 JSON) ;
`docker-compose.prod.yml` (app + Postgres 17, base jamais publiée) ;
migrations au démarrage ; `GET /api/health` en health check ;
`deploy/install.sh`, `deploy/backup.sh` (dump `pg_dump` en archive tar,
chiffrement `age` optionnel par clé publique, rotation),
`deploy/restore.sh` (refuse une base non vide sans `--force`, et
sauvegarde alors d'abord ce qu'il remplace) et `deploy/prod-deploy.sh`,
la boucle cron de Devotion (sauvegarde, sinon pas de déploiement) ;
`docs/install.md`, `docs/backup.md`. Vérifié : image construite, santé
200, front servi, sauvegarde → volume supprimé → restauration sur base
vide → données revenues et santé 200 (aussi en chiffré, et sur une
machine vierge). **Reste, pour Romain** : choisir l'hôte (proposition
dans `docs/install.md` : un conteneur dédié plutôt que PCT 101), y
faire la première installation, ajouter la route sur le Traefik de
PCT 100 et l'enregistrement DNS chez Ionos, installer la boucle de
déploiement sur l'hôte Proxmox — puis le test du téléphone en 4G, qui
attend aussi `session/invite-and-join`. `prod-deploy.sh` n'a pas pu
tourner sans conteneur cible.

---

### `platform/connect-claude-mcp` · doing — reste l'essai réel depuis Claude Desktop

**Pourquoi** — Romain veut préparer sa campagne en parlant à son propre
Claude (Claude Desktop ou Claude Code sur son Mac), qui modifie
directement la campagne dans Promptus.

**Périmètre** — Jetons personnels du MJ : créés, nommés, listés et
révoqués depuis son espace, montrés une seule fois, stockés hachés,
acceptés par le serveur en en-tête `Authorization` sur les routes de
préparation seulement (pas d'invitation, pas de compte, pas de soirée).
Un serveur MCP local (lancé par Claude sur le Mac, il parle à Promptus
avec le jeton) : lister les campagnes, lire une campagne ou une scène,
lire les alertes de cohérence, appliquer des modifications ciblées,
exporter et importer le fichier de campagne. Mêmes garde-fous que
l'écran : modifications validées par le serveur, tout ou rien ; rien
n'atteint les joueurs sans la validation du MJ dans Promptus. Une page
d'aide dit comment le déclarer dans Claude.

*Hypothèse* — Claude tourne sur le Mac de Romain. Claude sur le web
demanderait un serveur joignable depuis internet avec OAuth : hors
périmètre.

**Étapes** — Jetons (table, création, révocation, garde) → serveur MCP
et ses outils → page d'aide et réglage dans l'espace MJ → essai réel
depuis Claude Desktop.

**Fini quand** — Depuis Claude Desktop, Romain demande « ajoute un
indice vers le quai dans la scène de l'auberge », la scène change dans
Promptus, et un jeton révoqué est refusé.

**Risques** — Un jeton qui fuit donne la main sur les campagnes : portée
limitée à la préparation, révocation immédiate, jamais affiché deux
fois.

**Origine** — Demande de Romain, 8 octobre 2026.

*État* — Depuis son accueil MJ, Romain crée un jeton nommé « pour
Claude », le copie (il n'est montré qu'une fois), voit quand chaque
jeton a servi pour la dernière fois et le révoque d'un clic ; un jeton
révoqué est refusé à l'appel suivant. Un jeton ne sert qu'à la
préparation : lister les campagnes, lire une campagne, exporter,
réimporter par-dessus une campagne existante, appliquer des
modifications ciblées, lire la jauge des actes. Il ne peut ni créer de
jeton, ni inviter, ni toucher à la table, à la soirée ou à quoi que ce
soit qui dépense le budget IA, ni déclarer la campagne jouable : ce
geste reste celui du MJ dans Promptus (les alertes du validateur
reviennent à chaque lecture et chaque modification). Le serveur MCP
local donne à Claude sept outils décrits en français (lister, lire une
campagne, lire une scène, alertes de cohérence, modifier l'histoire,
exporter, importer) ; une modification refusée n'applique rien et Claude
lit pourquoi. Le même panneau montre la configuration à coller dans
Claude Desktop et la commande pour Claude Code, jeton compris juste
après sa création. Testé de bout en bout côté serveur et côté outils
(appels simulés) ; pas encore essayé depuis le vrai Claude Desktop.
*Décision ouverte* — faut-il refuser les écritures par jeton pendant une
soirée en cours ?

---

## Épic `ui`

Le design system dessiné sur le canevas (`MEMORY.md` §2), en composants
React partagés par le téléphone, l'ordinateur, la tablette et la TV.
Chaque composant du canevas (« Composant — … ») a son équivalent ici.

### `ui/write-design-tokens` · doing — reste la relecture de /reference par Romain

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

**État** — Tokens dans `front/src/styles/tokens.css` (`@theme static`),
valeurs relevées sur `Main.dc.html`, `kit.py`, `Bouton`, `GameCard` et
`Gemme` : gris de la table, ivoire et encre, six couleurs de stats
(formes documentées et dessinées en petite grille), rouge des dégâts,
polices, échelle de texte, rayons, ombres dures, durées, courbes et
animations ; matières en `@utility` (`surface-table`, `surface-slab`,
`button-card`, `button-card-dark`, `card-frame`, `material-common` à
`material-divine`, `damage-number`). La palette par défaut de Tailwind
est retirée : aucune teinte ne peut entrer dans l'interface par erreur.
Les variables shadcn pointent sur ces tokens (ivoire = action
principale, rouge des dégâts = destructif) ; `<html class="dark">`,
aucun thème clair. Les trois polices sont servies par l'app via
`@fontsource` (sous-ensemble latin, œ compris), sans CDN. Animations
réduites : une règle globale retire animations et transitions, chaque
élément animé a un état de repos calme (reflet hors carte, dé posé,
chiffre lisible) ; le mouvement piloté en JS lit
`usePrefersReducedMotion` (testé sur le dé qui roule : il affiche son
résultat tout de suite au lieu d'une face au hasard). Page de
référence : `/reference`. Lint, typecheck, knip, tests et build
passent dans PCT 105. **Reste** : un coup d'œil de Romain sur
`/reference` dans un navigateur, animations réduites activées et
désactivées — la page n'a été vérifiée que par le build et les tests.

### `ui/build-game-components` · doing — reste l’essai sur un vrai téléphone

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

**État** — Douze composants dans `front/src/components/game/`, chacun
fonction de ses props, sans appel réseau, styles portés des planches
dans `game.css` (classes `gk-`, couleurs lues dans les tokens) :
`StatGem` (grille 7/9/11 cases, vague de lumière propre à chaque stat),
`Hearts`, `CellBar`, `ThreatClock`, `CardButton` (ivoire / noir, bascule
à l’appui, reflet, enfoncé, désactivé), `ArcadeCluster`, `GameCard`
(action, indice, scène ; six raretés ; en main, choisie, injouable, de
dos), `ItemSlot` (les dix sprites 12 × 12 du canevas, cadre par rareté,
quantité, case choisie), `ConditionBadge` (huit icônes, aide / gêne,
tours, tampon), `StatusBanner` (sept tons), `Toast` + `ToastStack`
(trois tons, glisser à droite ou bouton « Ranger »), `BottomPanel`
(tiroir Base UI qui monte du bas, se ferme en glissant ou par Échap).
Tout est montré avec ses états dans `/reference` (section à part,
`GameComponentsSection.tsx`, bouton « Rejouer les effets »).
Choix : les cœurs suivent le composant du canevas et `MEMORY.md` —
2 PV par cœur, dix cœurs au plus, au-delà de 20 PV chaque cœur vaut
max / 10 (la note « au-delà de 40 PV » de la planche des pistes est
dépassée) ; `count` permet moins de cœurs quand la place manque. Les
raretés reprennent les noms de `Rarity` (`shared/src/story/model.rs`),
le badge prend nom et genre (`boon`/`bane`) de `ConditionDef`. Coups,
cases dépensées, tampons et toasts jouent une fois (remonter le
composant pour rejouer) ; sous mouvement réduit, cœurs et cases montrent
directement l’état final. Tests : cœurs au-delà de 20 PV, cases
dépensées, rareté lue sans couleur (losanges + nom), un essai par
composant. Reste pour Romain : regarder `/reference` sur son téléphone,
en mouvement réduit aussi, et dire si les rendus collent aux planches.

### `ui/roll-faceted-dice` · doing — reste les 60 images/s sur un vrai téléphone

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

### `ui/offer-light-theme` · doing — reste la relecture de Romain sur son écran

**Pourquoi** — Romain lit mal le texte clair sur la table noire. La
table noire reste l'identité de Promptus, mais chacun doit pouvoir
choisir un thème clair sur son appareil.

**Périmètre** — Un réglage « Thème : sombre / clair / comme
l'appareil », retenu par appareil, sombre par défaut, accessible au MJ
et au joueur. Une palette claire tirée des mêmes jetons : table blanc
cassé, texte encre, cartes blanches à bord gris, couleurs des
statistiques foncées d'un cran pour rester lisibles sur fond clair.
Suivent le thème : les matières des cartes et gemmes, la carte de
combat dessinée, la carte du monde en hexagones. Ne changent pas : le
pixel art (personnages, tuiles, objets) et l'écran TV, qui reste sombre.

**Étapes** — Jetons clairs sous `[data-theme="light"]` → couleurs en
dur passées en jetons (matières, carte, hexagones) → réglage et
mémorisation → vérification écran par écran dans les deux thèmes.

**Fini quand** — Chaque écran (MJ, joueur sur téléphone et ordinateur,
carte, voyage, `/reference`) se lit dans les deux thèmes avec un
contraste d'au moins 4,5:1 pour le texte courant, et Romain le
confirme sur son écran.

**Risques** — Une couleur en dur oubliée laisse une tache noire dans le
thème clair ; les couleurs vives des statistiques perdent du contraste
sur fond clair.

**Origine** — Demande de Romain, 8 octobre 2026.

**État** — Un réglage « Thème : Sombre / Clair / Comme l’appareil »
sur l’accueil du MJ (à côté de « Se déconnecter ») et chez le joueur
sous l’entrée des règles (onglets Perso et Journal au téléphone,
colonne de gauche à l’ordinateur). Retenu par appareil, sombre par
défaut, appliqué avant le premier affichage (pas d’éclair noir) ;
« Comme l’appareil » suit le réglage du téléphone en direct. Palette
claire : table blanc cassé, texte encre, panneaux blancs, cartes
blanches à tranche grise, ombres légères ; couleurs des stats foncées
d’un cran, toutes au-dessus de 4,5:1 sur chaque surface claire ; les
gemmes, les dés et le bandeau « à toi » gardent leur couleur vive et
leur chiffre encre. Suivent le thème : tables, panneaux, jauges vides (cœurs,
cellules, horloges), bandeaux, toasts d’information, la marge autour de
la carte de combat, le fond et le brouillard de la carte du monde, les
lignes des ennemis en combat. Restent noirs, comme des pièces de jeu :
carte noire et dos de carte, cases d’objet, badge d’état néfaste,
toast de mauvaise nouvelle, bouton d’arcade sombre, et tout l’écran TV.
Le pixel art et l’intérieur des cartes ne changent pas. Typecheck,
lint, knip et tests passent (sauf les 9 tests connus de la table du MJ
qui échouent sous Node faute de `localStorage`). **Reste** : Romain
relit chaque écran dans les deux thèmes sur son écran.

---

### `ui/adopt-pixel-menu` · todo

**Pourquoi** — Romain veut une interface entièrement pixel art : la
piste « Menu pixel » de la planche des pistes d'interface (écartée au
départ) devient la direction retenue.

**Périmètre** — Toute l'interface, MJ et joueur, dans les deux thèmes :
police pixel pour les titres, boutons et étiquettes (Silkscreen ou
équivalent), coins en escalier, relief en aplats, curseur façon jeu de
rôle rétro sur le choix actif, panneaux et champs au même dessin. Le
texte long (narration, notes, fiches) garde une police lisible. Les
pièces de jeu déjà en pixel art restent telles quelles.

**Étapes** — Jetons et composants de base (bouton, panneau, champ,
onglet) → écrans MJ → écrans joueur (téléphone d'abord) → page de
référence du design → vérification écran par écran dans les deux thèmes.

**Fini quand** — Chaque écran suit la piste Menu pixel dans les deux
thèmes, le texte courant garde un contraste d'au moins 4,5:1, et Romain
le confirme sur son écran.

**Risques** — Une police pixel est illisible en petit sur téléphone ;
la mémoire du projet disait l'inverse (UI noir et blanc, pixel pour le
jeu seulement) : à mettre à jour.

**Origine** — Demande de Romain, 8 octobre 2026.

---

## Épic `engine`

Le moteur de règles, dans `shared/`, pur et sans base de données.
Le serveur est seul juge (`MEMORY.md` §3). Spécification : le code et
les tests unitaires de `src/lib/engine/` du V1
(`archive/promptus-v1-nextjs.zip`), à porter test par test.

### `engine/model-rule-system` · doing

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

**État** — Fait côté moteur, à valider par Romain. Le modèle vit dans
`shared/src/rules/` (`model.rs`, chargé et validé par `load.rs`) et le
format est décrit dans `docs/rules-format.md`. Un système est un fichier
YAML (`content/rules/<id>/v<n>.yaml`) avec un id et une version :
caractéristiques et formule du modificateur, CA et PV en formules (où
`/` arrondit vers le bas), dé de test, difficultés nommées, les quatre
bandes de résultat et ce qu'elles donnent (XP, dégâts doublés), jet de
groupe, règle d'attaque (quelle stat principale, la précision compte ou
non), économie d'actions par contexte (`sol` ; `vaisseau` avec « 1
attaque max »), sens des recharges, décompte des durées, progression, ce
qui arrive à 0 PV (KO puis hors combat, ou jets contre la mort), classes,
objets, états, situations, ressources, paliers et fiches d'adversaires.
Une seule forme d'action pour les cartes de classe, les attaques des PNJ
et l'usage des objets. Le chargement refuse les références cassées avec
un code, un chemin et un détail. Les deux brouillons, `corsaires/v1.yaml`
et `brasier/v1.yaml`, transcrivent `dnd-save/` tel que joué, défauts
compris (deux modèles de dégâts, précision qui ne compte pas, CA 14 des
gardes, Canonnier à 63 points, renvoi à `Combat_Sol.md`) ; chaque
lecture imposée par le modèle est marquée `INTERPRETATION` dans le
fichier. Un test modifie le fichier (dégâts, recharge, actions par tour)
et voit le jeu changer sans code. Reste : un combat complet de chaque
monde (`engine/run-combat`) ; le mode « jet sous la compétence » (d100)
du V1 n'est pas porté, aucun monde ne s'en sert ; le Brasier n'a encore
aucune fiche d'adversaire au sol (la source n'en a pas).

**Origine** — V1 `ruleset.ts`, `ruleset-schema.ts` · planche « Règles »
(moments 1 à 4 et 6) · `dnd-save/DnD-16-05-2026/regles_*.md`

### `engine/lint-rule-system` · doing — reste l'affichage au MJ et la relecture des seuils par Romain

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

**État** — Contrôle pur dans `shared/src/rules/lint/` :
`rules::lint(&système)` (ou `lint_with` pour une campagne donnée) rend
des problèmes au format commun avec le validateur de campagne
(`promptus_shared::issue::Issue` : gravité erreur/avertissement/info,
code stable, chemin, détail anglais, message français pour le MJ), sans
jamais empêcher le chargement. Treize contrôles, listés dans
`docs/rules-format.md` § Lint. Sur les Corsaires : les deux modèles de
dégâts (PNJ et pistolet du marché noir), « précision » qui ne change
aucun jet, la stat principale ambiguë (6 classes), les gardes à CA 14
et Gueule-Rouge à 12 hors table, trois PNJ dont la CA ne suit pas
10 + DEX, le Canonnier à 63 points, « épée standard » et « jets de
soin » non définis, deux « Pistolet à silex ». Sur le Brasier : le
renvoi à `Combat_Sol.md` (erreur), la précision, la seule attaque à bord
contre deux au sol (info, à confirmer), le sens des recharges jamais
écrit. Champ `exception: <raison>` ajouté aux PNJ. L'équilibre sort en
chiffres (`rules::balance_report`, imprimable) : dégâts attendus par
tour et par niveau contre chaque palier de CA, XP par session, niveau
final ; avec 4 sessions de 2 combats de 4 tours, Bretteur, Vigie,
Flibustier et Boucanier (Pilote et Mécano au Brasier) sont niveau 7
dès la session 3, et les deux Canonniers font moins de la moitié des
dégâts des autres. Tests sur les deux brouillons et sur leurs copies
corrigées. Reste : montrer ces problèmes au MJ (aucune route ni écran
ne les expose encore), et que Romain valide les seuils et les
paramètres de campagne par défaut.

**Origine** — `docs/lecons-des-parties.md` §2 et §3

### `engine/roll-checks` · doing

**Pourquoi** — Chaque demande d'un joueur finit en jet ou en refus
expliqué.

**Périmètre** — Dés et formules, avantage et désavantage, critiques,
test contre une difficulté, jet de groupe (la moitié suffit), tirages
côté serveur avec une source de hasard injectable pour les tests.

**Fini quand** — Tests `dice.test.ts` et `skill-check.test.ts` du V1
portés et verts.

**État** — Fait côté moteur, à valider par Romain
(`shared/src/rules/{dice,check}.rs`). Expressions `NdX+M` ou valeur fixe,
bornées ; dé de test avec modificateurs de caractéristique et d'états ;
avantage et désavantage seulement si le système les a (les deux
brouillons non : une demande est refusée), et ils s'annulent ; les quatre
bandes, le naturel passant avant le seuil ; jet de groupe (au moins la
moitié — règle reprise de la planche « Voyager », absente des parties,
à confirmer) ; chaque jet rend son détail (faces, face gardée, chaque
modificateur et sa source, total, cible, bande). Hasard injectable :
`SeededDice` (ChaCha8, reproductible) pour le serveur, `ScriptedDice`
pour les tests. Intentions de `dice.test.ts` et des tests de jet du V1
portées, sur les deux mondes.

**Origine** — V1 `dice.ts`, `skill-check.ts` · planche « Voyager »
(jet de groupe)

### `engine/resolve-actions` · doing

**Pourquoi** — Une carte jouée doit produire le même effet chez tous,
calculé une seule fois.

**Périmètre** — Le résolveur déclaratif du V1 : 20 effets primitifs,
catalogue d'actions, résolution d'attaque (toucher, dégâts, critique),
bonus déjà calculés pour l'affichage des cartes.

**Fini quand** — Tests `resolver.test.ts` du V1 portés et verts.

**État** — Fait côté moteur, à valider par Romain
(`shared/src/rules/action.rs`). `resolve_action` prend une scène et rend
la suivante avec ce qui s'est passé, ou un refus qui ne change rien :
tour du joueur, état qui empêche d'agir, niveau, recharge, budget
d'actions et limite par type, objet manquant, cibles (nombre, camp, à
terre), situation inconnue, choix ou difficulté manquants. Puis jet pour
toucher (ou jet opposé, ou touche / critique automatique), dégâts fixes
ou aux dés avec leur détail, critique, soins, états avec jet de
résistance, effets « au toucher », recharge, XP selon la bande, objet
consommé. Les cartes d'un personnage se dérivent de sa classe, de son
niveau et de son emplacement libre, bonus d'attaque déjà calculé. Testé
sur les deux mondes ; intentions de `resolver.test.ts` portées (dégâts
bornés à 0, soins bornés au max, état posé avec sa durée, résistance en
cascade, attaque touchée, ratée, critique). Les 20 effets primitifs du
V1 ne sont pas repris tels quels : les effets de monde (scènes, indices,
musique) n'appartiennent pas au moteur de règles.

**Origine** — V1 `resolver.ts`, `catalog.ts`

### `engine/apply-conditions` · doing

**Périmètre** — Les états définis par le système de règles (étourdi,
apeuré, immobilisé, empoisonné, renversé… ; les 14 du SRD le jour où il
existe), leur durée en tours, leurs effets
sur les jets et le mouvement ; ce qui doit être montré au joueur (badge
et effet pixel).

**Fini quand** — Les états des deux mondes s'appliquent dans les tests
du moteur ; tests `conditions.test.ts` du V1 portés.

**État** — Fait côté moteur, à valider par Romain
(`shared/src/rules/conditions.rs`). Les états sont définis par le
système (étourdi, renversé, apeuré, immobilisé, empoisonné, désorienté,
saignement…) ou en ligne sur une carte ; leurs effets portent sur les
jets, l'avantage, la précision, les dégâts donnés et reçus, les
caractéristiques, le mouvement, le tour perdu, les dégâts par tour.
`start_turn` fait avancer les recharges et vide le budget d'un
combattant qui perd son tour ; `end_turn` applique les dégâts par tour,
décompte les durées du porteur et passe en hors combat un KO resté 3
tours sans soin. Chaque état garde ce que montre le badge (nom, bienfait
ou malus, tours restants). Intentions de `conditions.test.ts` portées
sur les états des deux mondes ; les 14 états du SRD sont dans le
préréglage `srd` (`engine/add-srd-preset`).

**Origine** — V1 `conditions.ts` · planche « États des personnages »

### `engine/run-combat` · doing

**Pourquoi** — Le combat est le moment où les règles se voient le plus.

**Périmètre** — Initiative, ordre du tour, tour d'un combattant
(mouvement, action, fin du tour), attaque avec portée et ligne de vue
lues sur la grille (`maps/model-grid-maps`), mort d'un adversaire, fin
du combat et expérience gagnée.

**Fini quand** — Tests `combat.test.ts` et `grid.test.ts` du V1 portés ;
un combat de la démo se joue entièrement dans les tests du moteur.

**État** — Fait côté moteur, à valider par Romain. Le combat vit dans
`shared/src/combat/` : un combat est un état pur (positions sur la
carte, ordre d'initiative, round, tour actif, qui est encore dedans) et
chaque commande — se déplacer, agir, fuir, finir son tour, arrêt par le
MJ — rend le combat suivant et ses événements, ou un refus sans rien
changer. Initiative lue dans le système (égalités : joueurs d'abord ou
relance) ; déplacement payé par l'action « Se déplacer », chemin vérifié
case par case ; portée, ligne de vue (brouillard compris) et couvert lus
sur la grille, le couvert entrant dans le jet ; zones, lignes et rafales
au contact résolues depuis la grille ; adversaire à 0 PV vaincu et
retiré, personnage KO resté au sol puis hors scène après 3 tours, fuite
(jet de DEX si le MJ le demande) ; fin quand un camp n'a plus personne
debout, avec l'XP gagnée par chacun. Le format gagne un bloc `combat`
(action de déplacement, de fuite, couvert −2/−5, longue portée −2, rayon
de zone) et `range`/`long_range` sur les actions, documentés dans
`docs/rules-format.md`. `run_fight` joue un combat entier avec une
politique par camp (base pour `engine/simulate-fights`). Tests :
`combat_grid.rs` (intentions de `combat.test.ts` et `grid.test.ts` ; la
géométrie pure déjà portée reste dans `maps_rules.rs`) et
`combat_worlds.rs` — la bagarre du quai de Port-Louis jouée jusqu'au
bout sur 20 graines avec la tactique de Gueule-Rouge (les joueurs
gagnent les 20, en 2 à 7 rounds, des marins fuient), et la coursive du
Cure-Dent contre six Vorr provisoires écrits dans le test (9 victoires
sur 20 en fonçant un par un dans le sas). À valider par Romain, marqué
`INTERPRETATION` : 6 cases par déplacement quand le système ne dit rien
(D&D 5e), les portées en cases ajoutées aux actions des deux mondes
d'après leur description, couvert et longue portée par défaut, un
adversaire à 0 PV vaincu sur-le-champ, un corps au sol enjambable.
Le combat de démo gobelins du V1 est porté sur le préréglage SRD
(`engine/add-srd-preset`). Reste : aucune XP de victoire séparée (les deux
mondes n'en donnent pas) ; la lumière (nuit) ne gêne pas encore la vue ;
les Vorr réels viendront de `campaign/rewrite-two-worlds`.

**Origine** — V1 `combat.ts`, `grid.ts`

### `engine/level-up` · doing — reste un passage de niveau joué pour de vrai, sur téléphone

**Périmètre** — Niveaux par expérience ; points de vie au dé de vie ou à
la moyenne, au choix du joueur ; nouvelles cartes de classe.

**Fini quand** — Borin passe niveau 4 avec ses deux options de PV et sa
nouvelle carte, comme sur la planche.

**Origine** — Planche « Entre deux » (moments 1 et 2)

**État** — Livré et vérifié (`cargo test --workspace`, clippy, `tsc -b`,
ESLint, Vitest). Le niveau vient toujours de l'XP. Un système de règles
peut dire que chaque niveau au-delà du premier ajoute des PV : le dé
(lancé par le serveur) ou la moyenne, plus le modificateur d'une
caractéristique, au choix du joueur, une seule fois par niveau, jamais
relancé ; l'XP reprise reprend les PV de ses niveaux. Un niveau débloque
les cartes de classe qui l'attendaient. Sur le téléphone, onglet Perso,
une carte « Niveau 4 » s'affiche tant que le joueur n'est pas passé par
ce niveau : les deux options de PV niveau par niveau, la nouvelle carte,
puis « C'est noté » ; le MJ lit « Niveau 4 : 19 PV max (+9) » dans
l'historique des fiches. Le test du moteur rejoue Borin au niveau 4
sur les deux mondes témoins. Hypothèse posée : les règles transcrites
des Corsaires et du Brasier ne donnent pas de PV par niveau, elles sont
gardées telles quelles ; sur ces mondes un niveau apporte ses cartes
(niveaux 3 et 7), et les deux options de PV arrivent dès que le MJ les
écrit dans ses règles (`progression.hit_points_per_level`, éditeur de
règles).

### `engine/save-against-death` · doing — reste une mort jouée pour de vrai à une soirée

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

**État** — Livré et vérifié (`cargo test --workspace`, clippy, Vitest). Deux règles
de 0 PV, données du système : `knocked_out` (les deux mondes témoins :
inconscient, hors combat après 3 tours sans soin) ne propose jamais de
mort ; `death_saves` garde le personnage mourant dans le combat — son
tour est un jet contre la mort, 1 et 20 naturels, un coup reçu compte un
échec (deux sur un critique), un soin relève tout, un allié à côté peut
le stabiliser — et trois échecs **proposent** la mort, cachée aux
joueurs, que le MJ confirme ou remplace par « stabilisé ». Sous les deux
règles, le MJ peut décider la mort d'un personnage à 0 PV, depuis le
combat ou les fiches, en deux gestes. Le mort passe « tombé » : sa fiche
reste pour la chronique, son jeton quitte la carte, le journal de la
table le dit, il ne reçoit plus d'XP du combat. Les sept moments de la
planche se rejouent dans `shared/tests/character_fate.rs` sur les deux
mondes (sous `death_saves` écrit par le MJ dans ses règles), le parcours
serveur dans `back/tests/fate_test.rs`, qui passe par l'éditeur de règles
(brouillon, texte, verrouillage) avant de jouer le combat. Le nombre de
réussites et d'échecs vient des règles, le téléphone dessine autant de
cases.

### `engine/formalise-house-rules` · doing — reste une vraie règle maison formalisée avec la clé OpenRouter, puis jouée à une soirée

**Pourquoi** — Un MJ écrit sa règle en français ; le serveur doit
pouvoir la juger.

**Périmètre** — Le co-MJ traduit une règle maison en règle formelle
(déclencheur, effet, exceptions, ce que voient les joueurs) avec des
cas de test ; le MJ relit et valide. Format formel à définir sur les
effets primitifs existants.

**Fini quand** *(hypothèse de spécification, octobre 2026 : la lecture
la plus probable de la planche « Règles », moment 5, et des deux mondes ;
à corriger par Romain)* —
- une règle maison peut porter, à côté de son texte, une **forme
  formelle** dans le fichier de règles : un déclencheur pris parmi ce que
  le moteur sait déjà voir (une attaque qui touche, éventuellement en
  critique ou avec un type de dégâts ; une attaque ratée, éventuellement
  sur un 1 naturel), qui est concerné (camp, étiquettes de créature,
  exceptions), un effet fait des primitives existantes (un état posé à
  la cible ou à l'attaquant, des dégâts, un soin), ce que voient les
  joueurs (la règle, ou seulement son effet) et des cas de test ;
- le serveur l'applique en combat comme les autres règles, une seule
  fois par attaque (l'effet d'une règle maison ne redéclenche aucune
  règle maison), et le journal de combat le dit ; une règle « effet
  seulement » n'est jamais nommée aux joueurs, ni sur leur page des
  règles ni dans leur journal ;
- dans l'éditeur, le MJ écrit sa règle en français et demande au co-MJ
  de la formaliser : le co-MJ propose déclencheur, effet, exception et
  visibilité avec trois cas de test, que le serveur rejoue tout de suite
  (✓ ou —) ; un identifiant inventé par le modèle est retiré et signalé ;
  rien n'est enregistré avant que le MJ ajoute la règle au brouillon ;
- chaque enregistrement du brouillon rejoue les cas de chaque règle
  formalisée, et les combats simulés tiennent compte des règles ;
- cela marche sur les trois systèmes : « les morts-vivants craignent le
  feu » sur le SRD (moment 5 de la planche), « le pied qui glisse » sur
  les Corsaires et « l'arme qui s'enraye » au Brasier (le 1 naturel en
  attaque, que les deux mondes laissent aujourd'hui à l'improvisation).

**État** — Construit et testé, pas encore essayé avec un vrai modèle.
Une règle maison peut porter une forme `formal` dans le fichier de
règles (`shared/src/rules/house.rs`, format dans `docs/rules-format.md`
§ House rules) : quand (une attaque qui touche, en critique ou non, avec
un type de dégâts ; ou qui rate, sur un 1 naturel ou non), qui (camp,
étiquettes exigées, étiquettes et classes/adversaires exceptés), des
effets (un état sur la cible ou l'attaquant, des dégâts, un soin), ce
que voient les joueurs (`rule` ou `effect`) et des cas de test. Le
moteur l'applique dans `resolve_action`, une seule fois par cible, et
l'annonce par un événement « règle maison » ; une règle « effet
seulement » est coupée à la projection (page des règles, changements à
lire, journal de combat), son état ou ses dégâts restent visibles. Le
chargement refuse une forme qui nomme ce que le système n'a pas. Dans
l'éditeur, onglet « Règles maison » : « Formaliser avec le co-MJ »
(gabarit `house-rule.v1`, appel compté `rules.house_rule`) rend le
tableau Quand / Effet / Exception / Joueurs, le mot du co-MJ, les
identifiants inventés retirés et nommés, et les cas rejoués par le
serveur (✓ ou —, conforme ou non) ; le MJ peut montrer ou cacher la règle
aux joueurs, rejouer les cas, puis « Ajouter la règle » au brouillon.
Rien n'est stocké avant. Chaque enregistrement du brouillon rejoue les
cas de chaque règle formalisée (`report.houseRules`), et les combats
simulés tiennent compte des règles (le moteur les applique). Essayé sur
les trois systèmes dans les tests : « les morts-vivants craignent le
feu » (SRD, livrée avec le préréglage), « le pied qui glisse »
(Corsaires, renversé sur un 1 naturel) et « l'arme qui s'enraye »
(Brasier, étourdi sur un 1 naturel). Écarts : les déclencheurs se
limitent à l'attaque qui touche ou rate (pas de début ou fin de tour, de
mise à 0 PV, de test hors combat) ; le MJ corrige une proposition par la
case de visibilité ou dans l'onglet « Texte », pas encore champ par
champ ; le Brasier n'ayant pas d'adversaire, ses cas opposent deux
classes.

**Origine** — Planche « Règles » (moment 5)

### `engine/add-srd-preset` · doing — reste une campagne SRD préparée et jouée pour de vrai, et la relecture du fichier par Romain

**Pourquoi** — Beaucoup de MJ jouent à D&D 5e ; le SRD 5.1 est la partie
libre de ses règles, publiable sous licence Creative Commons.

**Périmètre** — Le SRD comme troisième système de règles, à côté des
deux mondes ; vérifier que le modèle le porte sans cas particulier.

**Fini quand** *(hypothèse de spécification, octobre 2026, d'après la
planche « Règles » (moments 1 et 6) et le combat de démo du V1 ; à
corriger par Romain)* —
- un préréglage `srd` (« D&D 5e · SRD 5.1 », attribution CC-BY-4.0)
  se charge et se propose à la création d'une campagne, à côté des deux
  mondes, puis s'édite comme eux (brouillon, version, verrou) ;
- il contient les six caractéristiques, la maîtrise qui grandit avec le
  niveau, l'avantage et le désavantage, les quatre classes de la planche
  (guerrier, rôdeur, roublard, magicien) avec leurs PV et leur CA
  propres, les peuples (nain, elfe, halfelin, humain), les 14 états du
  SRD, les jets contre la mort comme règle du 0 PV, les types de dégâts
  et des créatures étiquetées (gobelins, squelette, zombie…) ;
- tout cela passe par des champs **génériques** du format (aucun
  `if srd` dans le code), que les deux mondes peuvent aussi employer ;
- le combat de démo des gobelins du V1 se rejoue jusqu'au bout dans
  les tests du moteur et dans le rapport (`bun run rules-report`), et
  le contrôle des règles ne trouve aucune erreur dans le préréglage ;
- ce que le modèle **ne porte pas** du SRD est écrit noir sur blanc
  (dans le fichier et ici), plutôt qu'approché en silence.

**État** — Construit et testé. `content/rules/srd/v1.yaml` (« D&D 5e ·
SRD 5.1 », attribution CC-BY-4.0 en tête et dans `sources`, texte
français réécrit) est proposé à la création d'une campagne, s'édite et se
verrouille comme les deux mondes. Il apporte au format quatre champs
génériques et facultatifs, aucun cas particulier dans le moteur : un
bonus d'attaque qui suit le niveau (la maîtrise, `attack.bonus`, montrée
sur les cartes, la page des règles et le détail du jet), des PV et une
CA propres à une classe (dé de vie, armure de départ), des étiquettes de
créature et des types de dégâts. Contenu : six caractéristiques,
difficultés 5 à 30, avantage et désavantage, une action + une action
bonus + un déplacement par tour, longue portée en désavantage, les 14
états, les jets contre la mort comme règle du 0 PV, quatre classes
(guerrier, rôdeur, roublard, magicien), quatre peuples, trois objets
(torche, potion de soins, feu grégeois), six adversaires étiquetés
(gobelin, chef gobelin, squelette, zombie, loup, bandit). Le combat de
démo du V1 est porté : `content/scenarios/srd/embuscade-des-gobelins.yaml`
sur la carte `route-des-gobelins`, joué jusqu'au bout dans
`shared/tests/srd_preset.rs` et dans `bun run rules-report` (200 combats :
69 % de victoires des PJ en bagarreurs, 78 % en concentrés, 5 à 6
rounds) ; le contrôle des règles ne trouve aucune erreur ni
avertissement. **Pas porté** (écrit en tête du fichier et dans
`docs/rules-format.md`) : maîtrises de compétences et de sauvegardes,
achat de points et bonus des peuples (les caractéristiques viennent de
la classe), emplacements de sorts et repos (approchés par des
recharges), résistances et vulnérabilités (une règle maison peut en
exprimer une), sauvegarde pour moitié, modificateur ajouté aux dégâts
(écrit dans le montant), critique qui double tout, réactions,
concentration ; les jets contre la mort se jouent avec
`engine/save-against-death`. Pas de pack de thème ni de sprites SRD :
la carte se dessine avec les tuiles par défaut et les personnages avec
le premier pack.

**Origine** — Romain, 4 octobre 2026 (sorti du jalon 1)

### `engine/simulate-fights` · doing — reste le recalage du temps et la relecture par Romain

**Périmètre** — Simuler N combats avec les fiches réelles et les
monstres d'une rencontre, sous une version des règles ; taux de victoire,
durée en tours et en minutes estimées à six joueurs, dégâts par classe,
effet d'une règle changée (comparer deux versions). C'est ce qui aurait
permis de roder le combat de vaisseau du Brasier avant de le jouer.

**Fini quand** — Le rapport de la phase 1 simule la bagarre du quai et
un combat au sol du Brasier, et compare deux versions d'une règle.

**État** — Fait côté moteur, à valider par Romain. Une rencontre est un
scénario en données (`content/scenarios/<monde>/<id>.yaml`) : carte,
contexte, portes, les deux camps (classe et niveau, ou fiche
d'adversaire ; case de départ), tactiques, moral (qui fuit, quand),
graine, nombre de combats et variantes de règles. Une variante est une
liste `chemin: valeur` appliquée en mémoire au brouillon avant son
chargement (`attack.precision`, `cooldowns.meaning`,
`adversaries[gueule_rouge].actions[…].tags[0].damage.amount`…) : le
fichier de règles n'est jamais modifié. `combat::simulate` joue N
combats sur des graines suivies et rend, de façon reproductible et en
JSON : victoires, défaites, nuls et arrêts ; rounds et minutes estimées
(moyenne, médiane, min–max) ; par classe et par fiche d'adversaire,
dégâts infligés et reçus, taux de touche, KO, hors scène, fuites, XP ;
refus du moteur (toujours 0). `compare` donne l'écart d'une variante
sur les mêmes graines. Deux tactiques de référence, pas une IA :
« bagarreur » (le plus proche) et « concentré » (le plus faible à
portée, case de tir la moins chère, jamais arrêté dans une porte).
`bun run rules-report` (`--world`, `--n`, `--seed`, `--json`,
`--markdown`) imprime pour chaque monde le contrôle des règles,
l'équilibre, les combats et les comparaisons ; sortie complète dans
`docs/rapport-phase-1.md`. Sur 200 combats : la bagarre du quai est
gagnée à 98 % en bagarreur (3,3 rounds, ~23 min estimées), 94 % en
concentré ; compter la précision (+13 à +17 points de touche pour le
Bretteur et la Vigie) et passer Gueule-Rouge et ses marins en dégâts
fixes raccourcissent le combat et font remonter le concentré à 99–100 %.
L'abordage de la coursive passe de 86 % (bagarreur, entonnoir dans le
sas) à 100 % (concentré), 24 à 30 min ; lire « CD 1 » comme « pas deux
fois dans le même tour » ajoute 1,4 dégât par combat au Canonnier, seul
concerné, et efface son `DAMAGE_PER_TURN_LOW`. Sur le quai, le
Navigateur et le Chirurgien n'attaquent jamais ; dans la coursive, le
Xénologue, le Toubib et le Quartier-maître non plus (leurs cartes ne
font aucun dégât, à aucun niveau). Les Vorr sont les
fiches de la campagne du Brasier, recopiées au format des règles dans le
scénario (un test vérifie qu'elles n'en divergent pas) ; le moteur
exigeait ce que la campagne ne dit pas — type d'action, caractéristique,
portée du crachat (6 cases, inventée). À valider par Romain, marqué
INTERPRÉTATION : le modèle de temps (2 min de mise en place, 60 s par
tour de PJ, 30 s par tour d'adversaire, 5 s par tour vide), les dégâts
fixes arrondis à l'inférieur, les tactiques. Reste : recaler le temps
sur un combat chronométré ; non simulés, le crachat qui marque une cible,
l'initiative unique de l'escouade vorr et sa course vers le réacteur ;
le combat de vaisseau attend `engine/support-vehicle-combat`.

**Origine** — Planche « Règles » (moment 7) · `MEMORY.md` §6

### `engine/support-vehicle-combat` · doing — reste l'essai réel à six (Greyhound et essaim Vorr), la TV (`tv/show-evening`)

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

**Hypothèses posées (à corriger par Romain)** — L'abordage se tranche
sur le combat du pont : les assaillants l'emportent si leur camp gagne ce
combat ; un combat du pont arrêté par le MJ sans vainqueur les
repousse, et la bataille reprend. Le butin d'une scène de vaisseau se
distribue par le combat du pont (abordage) ; un navire qui amène son
pavillon sans abordage ne donne rien d'automatique, le MJ le donne à la
main. Le MJ peut faire agir n'importe quel membre d'équipage (LUMEN, ou
un joueur absent), sauf manœuvrer ou répartir l'énergie à sa place : il
le rassoit à un autre poste.

**État** — Construit de bout en bout :
- **Moteur** (`shared`) : un seul système de vaisseau, en données dans le
  bloc `vehicles:` des règles — coque, boucliers ou voilure, énergie ou
  équipage à répartir, postes, armes avec arc et portée, proue et angle
  mort, vent ou attraction, avaries au d6, moral, abordage. Le Cure-Dent
  (transcrit de `Combat_Vaisseau.md`) et le brick des Corsaires
  (inventé dans le même moule, à valider). L'interception du Greyhound
  et l'essaim Vorr sont écrits en scénarios et simulés avec des
  tactiques de référence ; les deux scènes existent dans les campagnes
  (le départ de l'acte 1 mène maintenant à l'interception).
- **Serveur** : le MJ ouvre le combat d'une scène ; les personnages
  validés forment l'équipage, assis par classe ; chaque joueur agit à
  son poste sur son écran, le serveur tranche tout sous le verrou de la
  campagne. Le tour ennemi se joue à la main ou se propose par le co-MJ
  sur une copie, validée telle quelle (refusée si le combat a bougé).
  L'abordage met le combat en pause et ouvre le combat du pont de la
  scène ; sa fin fait reprendre la bataille. À la fin, l'XP des jets de
  l'équipage est versée et l'issue de la scène va au journal du MJ.
- **Joueur (téléphone)** : l'onglet Carte devient le combat de vaisseau
  — à qui le tour, chaque vaisseau avec ce que l'équipage en sait (un
  ennemi reste inconnu tant qu'on ne l'a pas scanné), mon poste et ses
  actions, chacune visant ce que le serveur accepte (cible dans l'arc,
  case d'arrivée et proue, répartition de l'énergie, avarie), changer de
  poste, l'équipage, le journal ; pendant l'abordage, le combat du pont.
- **MJ** : un panneau « Combat de vaisseau » à côté de la carte — lancer,
  toutes les jauges, ajuster ou retirer un navire, asseoir l'équipage,
  faire agir LUMEN, manœuvrer (tap sur la carte) et tirer pour l'ennemi
  ou valider la proposition du co-MJ, aborder, arrêter.

Reste : faire tourner les tests serveur sur base (`battle_test.rs` et
les balayages de routes sont écrits mais n'ont pas tourné : le Postgres
de développement était hors service), jouer les deux combats à six pour
de vrai et les chronométrer (moins de 45 minutes chacun), et
l'affichage TV, qui vient avec `tv/show-evening`.

---

## Épic `campaign`

Le modèle de campagne du V1 (bible, fronts, nœuds, indices, entités,
état vivant), et les écrans de préparation du MJ.

### `campaign/model-story-graph` · doing — reste la relecture du format par Romain

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

**État** — Modèle livré dans `shared/src/story/` : une campagne est un
seul document (bible, actes, fronts à horloge, nœuds portant toute la
norme de scène, révélations et indices, PNJ, adversaires, lieux, objets,
factions, objectifs) aux identifiants stables dans un espace de noms
unique, plus l'état vivant du monde et ses opérations pures. Import et
export YAML d'une campagne entière, format décrit dans
`docs/campaign-format.md` ; une petite campagne de test en français
(`content/fixtures/phare-de-kerbrume.yaml`) passe le validateur sans
remarque et fait l'aller-retour à l'identique. Le validateur signale
sans bloquer : règle des trois indices, références cassées ou du
mauvais type, savoir requis donné nulle part ou seulement dans des
scènes facultatives, contrôles de structure. Côté serveur : table
`campaigns` (histoire et monde en JSONB typés, propriétaire `gm_id`),
verrou par campagne (`SELECT … FOR UPDATE` puis relecture), routes MJ
de création, liste, lecture, import, export et aperçu joueur, toutes
balayées par `gm_routes_test.rs` ; projection joueur unique, testée en
marquant chaque champ réservé au MJ. Tests V1 portés (validateur,
monde, concurrence). Reste à faire relire le format à Romain sur un
vrai monde (`campaign/rewrite-two-worlds`).

### `campaign/rewrite-two-worlds` · doing — reste la relecture des deux mondes par Romain

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

**État** — Les deux mondes sont écrits dans
`content/campaigns/<monde>/campagne.yaml` et se relisent comme du texte.
**Corsaires** : l'acte 1 réécrit (neuf scènes, dix PNJ, la boutique de
Dents-de-Fer, le combat du quai sur sa carte, musiques de `dnd-save/`
par ambiance, visuels décrits en pixel art). Les quatre informations de
l'acte 2 (route du Greyhound, escorte, réparation, courants) ont chacune
trois indices ou plus dans des scènes différentes, dont au moins deux
obligatoires ; six emplacements de personnage (un par classe jouée, le
Flibustier et le Boucanier reprennent les accroches du Bretteur et de la
Vigie) ont chacun des accroches dans au moins deux scènes ; les
difficultés sont celles du système ; les fiches chiffrées et les objets
renvoient au système de règles (`from_rules`) au lieu de recopier ses
nombres. Le validateur n'y trouve rien. La version jouée reste un cas de
test (`content/fixtures/corsaires-acte-1-joue.yaml`) : le validateur y
retrouve l'information critique seulement dans les lieux facultatifs,
l'absence d'accroches et les difficultés hors échelle. **Brasier** :
bible, quatre factions (rivalités et affinité de départ inventées), quatre
composants comme objectifs, le Cure-Dent et LUMEN, six emplacements de
classe, et une première scène jouable, « Le Toboggan » (six postes, six
jets, puis l'abordage Vorr sur la carte de la coursive) ; la suite de
l'acte 1 est une ébauche et le validateur le dit. Tout ce qui est
inventé est marqué « INVENTÉ — à valider par Romain ». Le Brasier n'a
aucune musique dans la source : morceaux à choisir, sans lien. Nouveaux
champs et contrôles (carte d'une scène, classe d'un emplacement,
`from_rules`, contrôles contre le système de règles et les cartes,
accroche par joueur et par acte) dans `docs/campaign-format.md`.
`bun run worlds` charge tout `content/` et imprime par monde le contrôle
des règles, l'équilibre, les cartes et le validateur d'histoire.
Importeur V1 (entités YAML + histoire JSON) et tests
`import-export.test.ts` portés sur la démo V1. Reste : que Romain relise
les deux mondes et tranche ce qui est inventé ; choisir les musiques du
Brasier ; porter les fiches Vorr dans le système de règles quand il
accueillera des adversaires au sol.

### `campaign/list-campaigns` · doing — reste l'essai réel de Romain

**Périmètre** — Liste des campagnes du MJ, création d'une campagne vide
(univers, préréglage de règles), réglages, budget IA.

**Fini quand** — Le MJ crée une campagne, la retrouve, la rouvre ; un
autre compte MJ ne la voit pas.

**Origine** — Planche « Préparer » (moments 1 à 3)

**État** — Côté serveur : la migration `006_campaign_settings.sql`
ajoute à la campagne le nombre de joueurs prévus, le budget IA (en
cents de dollar) et `archived_at` ; la liste porte ce qu'affiche une
carte (système de règles, joueurs installés, archivage, dernière
activité) ; nouvelles routes MJ `PUT …/settings`, `PUT …/archive` et
`GET /api/rule-systems` (les préréglages embarqués de `content/rules`
et leurs noms de statistiques) ; la création refuse un système de
règles inconnu. Côté MJ : l'accueil montre les campagnes en cartes (la
dernière travaillée en ivoire, les archivées à part), « Nouvelle
campagne » en trois étapes (titre et univers, préréglage de règles,
pitch, accroche, joueurs et budget IA), et la page de campagne
(`/campagnes/:id`) qui la rouvre : réglages modifiables, règles,
archiver ou rouvrir, accès à l'invitation (`/campagnes/:id/table`) et
à la vue des joueurs. Un test serveur vérifie qu'un MJ crée, retrouve
et rouvre sa campagne et qu'un autre compte MJ ne la voit ni dans sa
liste ni par son adresse (404). `bun run lint` et `bun run test`
passent. **Limites assumées** : le budget IA n'est qu'un réglage — rien
ne le consomme tant que `ai/count-ai-calls` n'est pas fait ; l'univers
est un texte libre (il ne choisit pas encore de pack de tuiles ou de
personnages) ; le préréglage n'est pas modifiable ici
(`campaign/edit-rule-system`). **Reste** : que Romain crée, retrouve et
rouvre une campagne pour de vrai, sur ordinateur et sur tablette.

### `campaign/review-story-graph` · doing — reste une campagne générée relue pour de vrai

**Pourquoi** — Le MJ relit et corrige tout ce que l'IA propose avant que
ça existe.

**Périmètre** — Écran de relecture : bible, fronts, graphe de nœuds et
indices, fiches ; atelier avec le co-MJ (diff à accepter ou refuser) ;
alerte de cohérence (règle des trois indices) ; validation de la
campagne qui la rend jouable.

**Fini quand** — Le MJ corrige une campagne générée, accepte un diff du
co-MJ, résout une alerte et la valide.

**Origine** — Planche « Préparer » (moments 6 à 8 et 10)

**État** — Livré et vérifié (`cargo test`, clippy, `bun run lint`,
`bun run test`). L'écran `/campagnes/:id/preparer` (bouton « Relire et
valider la campagne ») a cinq onglets : le **graphe** (scènes par acte,
une scène sélectionnée se corrige à droite avec ses indices, on en
ajoute), la **bible**, les **fiches** (PNJ, adversaires, objets), la
**cohérence** (les alertes du validateur en français, chacune avec
« Proposer une correction ») et **valider** (le bilan, puis
l'invitation). L'**atelier** du co-MJ reste à gauche : une demande
libre, sur la scène sélectionnée, ou une alerte ; le co-MJ lit toute la
campagne et ce que dit le validateur, propose des changements par
identifiant (`story::edit`), affichés en diff (« + Indice « … » →
Le quai ») ; les identifiants inventés sont écartés et comptés ; rien
ne change avant « Accepter », qui réapplique sur la campagne du moment
(une proposition dépassée est refusée). Chaque appel est compté dans le
budget IA. Une campagne n'ouvre une soirée qu'une fois **validée**, ce
qui demande zéro erreur ; un import est validé d'office (le MJ l'a
écrit), une campagne créée depuis un pitch ou générée attend le MJ.
Vérifié sur l'acte 1 joué des Corsaires : le co-MJ place les indices
manquants de la route du Greyhound et l'alerte disparaît.

### `campaign/edit-scenes-in-one-place` · doing — reste une scène préparée pour de vrai sur les deux mondes, puis jouée

**Pourquoi** — Romain se perd dans la préparation : une scène est
éclatée entre Préparer (texte), Cartes, Médias, et le combat, le son et
la musique n'ont aucun éditeur hors du fichier de campagne. Une scène
doit se préparer d'un seul endroit.

**Périmètre** — Dans Préparer, la scène choisie dans le graphe s'ouvre
en fiche à quatre onglets : **Texte** (titre, résumé, à lire à voix
haute, déroulé, notes MJ, indices, jets, sorties), **Combat**
(adversaires et nombre, tactique, moral, butin, XP ; « pas de combat »
possible), **Visuels** (image de la scène, carte de combat, vidéo
d'acte si c'est la première scène de l'acte : voir, générer, valider ou
refuser, comme dans Médias et Cartes), **Son** (humeur, bruits
d'ambiance, musique YouTube : titre, lien, recherche, écoute d'essai).
Chaque modification passe par les mêmes modifications ciblées que
l'atelier du co-MJ (tout ou rien, validées par le serveur). Les pages
Médias et Cartes restent pour la vue d'ensemble.

**Étapes** — Opérations de modification manquantes côté serveur
(combat, ambiance, musique) → fiche Scène et ses quatre onglets →
visuels branchés sur les médias et cartes existants → essai sur les
deux mondes.

**Fini quand** — Sur les Corsaires et le Brasier, le MJ prépare une
scène de bout en bout (texte, combat, image, carte, musique) sans
quitter la fiche, et la soirée en direct montre ce qu'il a réglé.

**Risques** — Un éditeur de combat trop libre laisse passer un
adversaire inconnu (le serveur doit refuser) ; la fiche devient trop
longue sur tablette.

**Origine** — Demande de Romain, 8 octobre 2026.

**État** — Livré et vérifié par les tests, pas encore essayé à la main
dans le navigateur. Dans Préparer, cliquer une scène du graphe ouvre sa
fiche à droite, avec quatre onglets. **Texte** : les champs d'avant,
plus ce qui lance la scène, la transition, les jets prévus (action,
caractéristique, difficulté, et ce que donnent une réussite, un échec,
un 1 et un 20) et les sorties vers les autres scènes ; les indices
restent en dessous. **Combat** : « Un combat » ou « Pas de combat »,
les adversaires choisis parmi ceux de la campagne et les PNJ qui ont
des caractéristiques, leur nombre, la tactique, le moral, l'issue, le
butin (objet, pièces, caché ou non) et l'XP ; une scène qui ouvre une
bataille navale garde ses navires et ne peut pas perdre son combat.
**Visuels** : l'image de la scène et, sur la première scène d'un acte,
sa vidéo d'introduction, à demander, garder ou refaire comme dans
Médias ; la carte de combat de la scène, avec son aperçu, à valider,
refuser, ouvrir dans l'éditeur, détacher, remplacer par une autre ou
générer pour la scène. **Son** : l'humeur, les bruits d'ambiance et les
morceaux YouTube (moment, titre, lien, recherche), avec un lien de
recherche et une écoute d'essai dans la fiche. Le serveur refuse
désormais une modification qui ferait combattre un adversaire inconnu,
mènerait vers une scène qui n'existe pas, donnerait un objet inconnu,
demanderait une caractéristique absente des règles ou mettrait un lien
qui n'est pas YouTube : rien n'est enregistré et le MJ lit pourquoi.
La jauge « prête à jouer » tient compte des cartes propres à la
campagne une fois validées. Vérifié : `cargo test` (règle de
modification, Corsaires et Brasier importés), Vitest de la fiche. Les
pages Médias et Cartes restent.

---

### `campaign/check-player-knowledge` · doing — reste une fin de soirée jouée pour de vrai

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

**État** — Livré et vérifié (`cargo test`, `bun run test`). À la
relecture, le validateur signale une information demandée par une scène
et donnée nulle part, ou seulement dans des scènes facultatives
(`KNOWLEDGE_NEVER_GIVEN`, `KNOWLEDGE_ONLY_OPTIONAL`, onglet Cohérence).
En fin de soirée, le panneau « Fin de soirée » liste ce que les scènes
suivantes demandent et que la table ne sait pas, avec où l'apprendre
encore, et « Ajouter au « Précédemment… » » glisse la ligne dans le
texte publié aux joueurs. Les retours de la séance le gardent. Vérifié
sur l'acte 1 des Corsaires tel que joué (`player_knowledge_test`) : la
soirée taverne → proposition → marché noir → quai laisse sans la route
du Greyhound, l'escorte et la réparation pour la transition vers
l'acte 2.

### `campaign/check-act-readiness` · doing — reste un acte préparé pour de vrai avec la jauge

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

**État** — Livré et vérifié (`cargo test`, `bun run test`). Une jauge
par acte (`story::readiness`, `GET /api/campaigns/{id}/readiness`), sur
l'écran de relecture (sous chaque acte du graphe, et en détail dans
« Valider ») : scènes complètes (résumé, lieu, ambiance, texte à lire,
déroulé, transition), informations à trois chemins et pas seulement
facultatives, une accroche par joueur, rencontres chiffrées avec
tactique, combats simulés (12 combats par rencontre, avec les règles et
les cartes de la campagne). Elle dit ce qui manque, le MJ décide.
Vérifié : l'acte 1 du Brasier n'est pas prêt (une scène à moitié
écrite, le secret du moteur donné une seule fois, l'abordage contre un
adversaire absent des règles) ; l'acte 1 des Corsaires tel que joué
signale la route du Greyhound seulement dans une scène facultative ;
l'acte 1 réécrit des Corsaires est prêt.

### `campaign/track-factions-and-goals` · doing — reste une vraie soirée du Brasier où les jauges bougent, et la relecture des règles d'affinité par Romain

**Périmètre** — Jauges d'affinité par faction, où gagner la faveur des
uns fait baisser celle de leurs rivaux ; objectifs de campagne à cocher
(les quatre composants du Brasier) ; un PNJ permanent joué par le co-MJ
(l'IA de bord LUMEN).

**Fini quand** (hypothèse écrite le 7 octobre 2026, à relire par Romain)
— Pendant une session, le MJ monte ou baisse d'un geste l'affinité d'une
faction ; un gain fait baisser d'autant chacun des rivaux **déclarés par
cette faction** (les rivalités ne sont pas forcément réciproques),
chaque jauge bornée par son `min`/`max` ; une perte ne fait monter
personne. Les joueurs ne voient que les factions dont ils ont entendu
parler (nom, description, jauge, rivaux connus ; jamais la `diplomacy`,
qui est le conseil du MJ, ni ses notes) : une faction devient connue
quand le MJ la fait connaître ou bouge sa jauge, ou quand l'objectif
qu'elle détient est atteint. Un objectif de campagne est caché, connu ou
atteint ; les joueurs voient les connus et les atteints, et qui le
détient si cette faction est connue. Un PNJ marqué `companion` dans la
campagne (LUMEN) se fait parler en un geste dans n'importe quelle scène,
en brouillon du co-MJ que le MJ relit. Vérifié sur les deux mondes : les
quatre races et les quatre composants du Brasier, la Couronne contre
Gueule-Rouge et les plans du Greyhound des Corsaires (qui n'ont pas de
compagnon).

**Origine** — `dnd-save/DnD_07-06-2026/Univers.md`

**État** — Le moteur (`shared/src/story/world.rs`) tient l'affinité, les
factions connues et l'état des objectifs dans le monde vivant (pas de
migration : le monde est un document, et un monde d'avant se recharge).
Côté MJ, l'écran de soirée a un bloc « Factions et objectifs » : jauge en
cases de part et d'autre du neutre, −1 / +1, « Faire connaître »,
objectif caché / connu / atteint ; chaque geste passe sous le verrou de
la campagne et laisse une ligne au journal (partagée pour ce que la
table voit, MJ seul pour la jauge d'un rival inconnu qui chute). Le
co-MJ reçoit les jauges, les objectifs et les compagnons dans son
contexte ; le bouton « Faire parler LUMEN » apparaît dès qu'un
compagnon existe. Côté joueur, l'onglet Journal montre les objectifs et
les factions connues avec leur jauge. Tests : le moteur sur les deux
YAML (bornes, rivaux, perte sans effet, objectif qui fait connaître son
détenteur), l'API sur les deux mondes (ce que voit Marc, journal
partagé ou non, gestes refusés, LUMEN sans scène), le balayage des
fuites avec une faction et un objectif inconnus marqués, et les deux
écrans en Vitest. **Hypothèses à confirmer par Romain** : les rivalités
du Brasier sont celles du YAML (inventées, notées dans `gm_notes`) ; un
point gagné coûte un point à chaque rival ; la `diplomacy` reste au MJ.
La TV ne montre pas encore les jauges (`tv/show-evening` montre le
journal partagé, où elles passent en une ligne).

### `campaign/edit-rule-system` · doing — reste une vraie modification jouée à la soirée suivante

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

**État** — Livré et vérifié (`cargo test`, clippy, `bun run lint`,
`bun run test`). Chaque campagne a ses versions de règles
(`rule_versions`, migration 015) : la première modification fait un
brouillon de la version jouée, numéro suivant, texte YAML commentaires
compris. L'écran `/campagnes/:id/regles` l'édite par onglets
(caractéristiques, jets et difficultés, actions et cartes, règles
maison, création) ou en texte ; chaque enregistrement recalcule ce que
ça touche : ce que les joueurs liront, les scènes de la campagne que ça
casse (adversaire disparu, difficulté hors échelle…), le contrôle des
règles, et les combats rejoués sur les deux versions avec les mêmes dés
— les scénarios du monde et les rencontres de la campagne
(`story::encounter_scenario`). Verrouillée, une version attend
l'ouverture de la prochaine soirée, jamais en cours de partie ; les
joueurs lisent alors ce qui change sur leur page des règles. Les règles
maison (`house_rules`) s'y affichent et le co-MJ les connaît.
L'historique compare deux versions (changements lus par les joueurs, et
le texte ligne à ligne). Pas encore : le co-MJ qui propose une
modification de règle (l'atelier arrive avec
`campaign/review-story-graph`).

---

## Épic `ai`

### `ai/route-llm-provider` · doing — reste la soirée du jalon jouée pour de vrai

**Pourquoi** — Changer de modèle ou de fournisseur sans toucher au jeu.

**Périmètre** — Trait de fournisseur (LLM, image, vidéo) avec
OpenRouter comme première implémentation ; gabarits de prompts
versionnés ; sorties validées par schéma ; faux fournisseur
déterministe pour les tests et le développement (comme `fake-llm.ts`).

**Fini quand** — Un appel réel à OpenRouter et le faux fournisseur
passent par le même trait ; une sortie hors schéma est rejetée avec une
erreur lisible.

**Origine** — V1 `llm.ts`, `prompt-template.ts`, `scripts/fake-llm.ts`

### `ai/evaluate-on-real-campaigns` · doing — reste une première évaluation avec un vrai modèle, puis un changement de modèle comparé

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

**État** — Livré et vérifié (`cargo test`). `bun run eval` rejoue
quatre cas (`content/evals/`) au modèle configuré : refuser l'offre de
Vaubernier, l'incident de Jacquot, le combat du quai à six contre six,
et une campagne générée depuis un pitch corsaire. Chaque cas a ses
critères écrits, en français, vérifiés par la machine : le bon PNJ
parle, ce que la table ignore n'est pas dit, la fiche est respectée,
aucun identifiant inventé, aucune formule de dés, du français ; pour la
génération, validateur sans erreur, règle des trois indices et standard
de scène (la jauge). Le passage est enregistré dans `evals/runs/` et le
rapport dit, cas par cas et critère par critère, ce qui est en progrès,
en recul, corrigé ou cassé depuis le précédent (`docs/evaluation.md`).
Pas encore de passage avec un vrai modèle : il faut la clé OpenRouter.

### `ai/count-ai-calls` · doing — reste la soirée du jalon jouée pour de vrai

**Pourquoi** — Chaque appel IA coûte (`MEMORY.md` §3).

**Périmètre** — Chaque appel LLM, image ou vidéo enregistré avec son
coût ; budget par campagne ; coût estimé avant chaque lot ; un lot qui
dépasserait le budget est refusé.

**Fini quand** — Un test prouve qu'un lot au-delà du budget n'émet aucun
appel.

**Origine** — `MEMORY.md` §3 · planche « Préparer » (coût estimé)

### `ai/generate-campaign` · doing — reste une génération réelle avec OpenRouter, relue par Romain

**Périmètre** — Pitch → bible, fronts, nœuds, indices, entités, en
tâches de fond suivies en direct ; rien n'est appliqué avant la
relecture du MJ (`campaign/review-story-graph`) ; identifiants inventés
écartés.

**Fini quand** — Un pitch produit une campagne valide (validateur vert)
que le MJ relit et applique ; tests `generation.test.ts` du V1 portés.

**Origine** — V1 `generation/pipeline.ts` · planche « Préparer »
(moments 4 et 5)

**État** — Livré et vérifié (`cargo test`, clippy, `bun run lint`,
`bun run test`). Sur une campagne pas encore validée, le bouton
« Générer à partir d'une idée » mène à `/campagnes/:id/generer` : le MJ
écrit l'idée, le ton, les thèmes, les contraintes et le format, voit le
coût au plus et ce qui reste du budget, lance. La tâche de fond suit le
pipeline du V1 : bible, actes, menaces et fiches (adversaires repris du
système de règles), puis scènes, révélations et indices, puis
vérification ; tant que le validateur trouve une erreur ou un défaut de
structure (trois indices, scène inaccessible…), le modèle propose des
corrections par identifiant, deux tours au plus, gardées seulement si
elles n'aggravent rien. Une réponse hors format est renvoyée une fois
avec ce qui n'allait pas. Les identifiants inventés sont écartés
(`story::prune`) et listés ; chaque appel est compté, la tâche est
refusée avant le premier appel si son estimation dépasse le budget, et
s'arrête si un appel le dépasserait. L'écran suit les étapes en direct ;
le brouillon dit ce qu'il contient et ce qu'en dit le validateur, et
n'arrive dans la campagne que sur « Appliquer », qui mène à la
relecture. Les tests du V1 sont portés (`generation_test.rs` : étapes
dans l'ordre, correction gardée, seconde chance sur un JSON invalide,
budget, brouillon appliqué une fois). Les cartes ne sont pas générées
ici : c'est `maps/generate-map-llm`.

---

## Épic `media`

### `media/play-youtube-music` · doing — reste la soirée du jalon jouée pour de vrai

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

### `media/draw-pixel-art-assets` · doing — reste la soirée du jalon jouée pour de vrai

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

### `media/generate-images-and-video` · doing — reste un vrai lot d'images et une vraie vidéo avec la clé OpenRouter, vus sur un téléphone

**Périmètre** — Les médias des actes écrits ou générés dans Promptus :
images pixel art par scène (`media/draw-pixel-art-assets`) et vidéo
d'introduction, en tâches de fond, stockées sur disque ; le MJ valide
ou relance chaque média ;
coût compté (`ai/count-ai-calls`). Vérifier d'abord le format de sortie
vidéo d'OpenRouter (`MEMORY.md` §4).

**Fini quand** — Une campagne générée a ses images et vidéos validées,
affichées sur les téléphones et la TV.

**Origine** — V1 `media/` · planche « Préparer » (moment 9)

**État** — Livré et vérifié (`cargo test`, clippy, `bun run lint`,
`bun run test`), sans clé réelle. Le format vidéo d'OpenRouter est
vérifié sur sa documentation (pas d'accès réseau à l'API depuis ici) :
`POST /api/v1/videos` rend une tâche, interrogée toutes les 10 s jusqu'à
`completed` (vingt minutes au plus), puis le fichier est téléchargé
depuis `unsigned_urls[0]` ; le coût vient de `usage.cost`. Modèle par
défaut `google/veo-3.1`, 8 s en 16:9, estimé 4 $ avant de partir
(`AI_PRICE_PER_VIDEO`). Le bouton « Images et vidéos » de la campagne
mène à `/campagnes/:id/medias` : chaque acte (sa vidéo d'introduction),
scène, PNJ, adversaire et lieu, avec ce qui est gardé, ce qui attend le
MJ (Garder / Refaire), ce qui se dessine, et pourquoi un essai a
échoué ; un mot de direction relance un sujet. « Dessiner les N
médias » lance en tâche de fond tout ce qui n'a rien en attente ni de
gardé, vidéos comprises si le MJ coche la case, avec le coût au plus
affiché et refusé d'un bloc s'il dépasse le budget. Un dessin coupé par
un redémarrage est marqué interrompu, jamais en attente sans fin. Les
fichiers sont servis par morceaux (`Range`), ce que le lecteur vidéo
d'un iPhone exige. Côté joueur, la vidéo gardée d'un acte passe au-dessus
de la scène dès qu'une scène de l'acte est ouverte. **Écart assumé** :
images et vidéos restent dans la base (`media_assets`), pas sur disque,
pour être sauvegardées avec elle par `deploy/backup.sh` sans second
chemin de sauvegarde ; à revoir si la base grossit trop. **Trouvé en
route** : la stack de production ne transmettait aucun réglage d'IA au
serveur (aucune fonction d'IA n'aurait marché) ; `OPENROUTER_*` et
`AI_PRICE_*` passent maintenant par `.env.production` (`docs/install.md`).
La TV reprend les images de scène approuvées (`tv/show-evening`) ;
pas encore la vidéo d'acte.

---

## Épic `session`

Le direct : comment une table se réunit et ce qui circule entre les
écrans. Invariants : projection joueur unique, temps réel sans données
de jeu, jetons hachés (`MEMORY.md` §3).

### `session/stream-live-changes` · doing — reste l'essai sur un vrai téléphone, et la route joueur

**Périmètre** — WebSocket Axum par session : « ceci a changé » (Postgres
NOTIFY relayé) et présence ; les clients relisent par l'API ;
reconnexion qui rattrape l'état.

**Fini quand** — Un téléphone coupé dix secondes revient sur l'état
exact de la table sans recharger la page.

**Origine** — V1 `realtime/`, `notify.ts`

**État** — Le canal est dans le serveur Axum (plus de serveur Node à
part). `GET /api/campaigns/{id}/live` ouvre la socket du MJ, derrière
`require_gm` (une campagne d'un autre MJ : 404 avant toute ouverture).
Chaque écriture suivie incrémente un compteur par sujet (`world`,
`story`, prêt pour `character:<id>`) dans `live_versions` (migration
`005`) et émet `pg_notify` **dans sa propre transaction** : rien ne part
si elle est annulée. `campaigns::save_world` et `save_story` le font,
donc `update_world` et la réimportation YAML. Une seule tâche du serveur
écoute Postgres (une connexion du pool, passé à 11) et relaie aux
sockets de la campagne par un canal `broadcast`. La socket ne transporte
que `{sujet, version}` et la présence (MJ connecté, identifiants des
joueurs), jamais de données : le client relit par l'API, donc par la
projection. Rattrapage : chaque (re)connexion commence par les versions
courantes de tous les sujets, et le client relit ceux qui ont bougé ; une
socket en retard sur son canal, ou toutes après une coupure de Postgres,
reçoivent `resync` avec les versions. Battement : le serveur pingue
toutes les 15 s et lâche un client muet depuis 45 s (sa présence
s'arrête) ; le client pingue toutes les 10 s, abandonne une socket muette
depuis 25 s, se reconnecte avec un délai doublé (0,5 s → 8 s), tout de
suite quand l'appareil revient en ligne ou la page redevient visible
(avec une sonde de 3 s si la socket a l'air encore ouverte). Côté
interface : `useLiveChanges(campaignId, onChange)`, l'indicateur
« Connexion… / En direct / Reconnexion… », et une première page qui s'en
sert, `/campagnes/:id/vue-joueurs` (ce que voient les joueurs, tenu à
jour en direct, avec le nombre de joueurs connectés). Le proxy Vite
transmet déjà les WebSockets (`ws: true`) et l'image de production les
sert à la même origine. Tests : 9 tests Rust de bout en bout (vrai
serveur TCP, vrais clients WebSocket) — deux clients reçoivent chaque
écriture avec une version croissante, une écriture annulée ne notifie
rien, un client coupé pendant trois écritures retrouve l'état exact, la
réimportation est son propre sujet, présence à l'arrivée et au départ,
client muet expulsé, client en retard → `resync`, écouteur coupé →
`resync` puis reprise, un autre MJ refusé (404, 401 sans session) ; plus
la reconnexion, le délai, le rattrapage et la sonde en Vitest avec une
fausse socket, et la page qui se rattrape après une coupure. **Reste** :
la route joueur `GET /api/play/{campaign}/live` est branchée par
`session/invite-and-join` (derrière `require_player`), et la création
d'un personnage touche `Topic::Character` ; reste à
appeler `live::touch` dans les prochaines écritures de
fiches ; puis l'essai réel : ouvrir `/campagnes/<id>/vue-joueurs` sur le
téléphone, couper le Wi-Fi dix secondes pendant qu'une écriture passe
depuis l'ordinateur, vérifier que la page revient seule à l'état exact.

### `session/project-player-view` · doing — reste à brancher carte, tour et journal quand ils existeront

**Pourquoi** — C'est l'invariant qui protège les secrets du MJ.

**Périmètre** — Un seul point de projection serveur pour tout ce qu'un
joueur (ou la TV) reçoit : pas de notes MJ, pas de cases non révélées,
pas d'objets cachés, ni nom ni PV d'un adversaire non révélé.

**Fini quand** — Tests `projection.test.ts` du V1 portés, plus un test
qui parcourt chaque route joueur et TV.

**Origine** — V1 `projection.ts` · `MEMORY.md` §3

**État** — Toute réponse aux joueurs est construite dans
`back/src/campaigns/projection.rs` (invitation, accueil du joueur, vue de
la campagne). Les routes joueur sont déclarées dans une seule liste
(`app::player_routes`, `app::invitation_routes`) que le routeur monte et
que `back/tests/player_routes_test.rs` parcourt : campagne marquée
`GMONLY<…>` sur chaque champ MJ, appel en joueur et en spectateur, aucune
marque, aucun PV caché, aucun id d'indice ; un joueur ne voit pas la
fiche d'un autre ; sans place à cette table (ou avec la session MJ) :
401. Ajouter une route joueur sans qu'elle soit balayée est impossible.
Vérifié en faisant fuiter exprès `/view` : le test casse. Des tests V1
sont portés ceux qui ont déjà leur matière (secrets, résumé, ids
d'indices, nom d'adversaire révélé ou non) ; la carte (cases et objets
cachés) est couverte par `Map::project` dans `shared`, mais aucune route
ne sert encore de carte, de tour ni de journal : leurs cas V1 (pions
dans le brouillard, « Adversaire 1/2 », PV masqués dans le journal)
arrivent avec ces routes, dans la même liste. Les routes de la TV
(`app::screen_routes`, `session/pair-shared-screen`) sont balayées de
la même façon par le même test, avec le jeton d'un écran jumelé.

### `session/invite-and-join` · doing — reste l'essai réel sur le téléphone de Marc

**Périmètre** — Lien d'invitation de la campagne avec message prêt à
coller sur Discord ; le joueur ouvre le lien dans le navigateur de son
téléphone, choisit un pseudo, puis créer, reprendre ou regarder ; jeton
secret sur l'appareil, empreinte seule côté serveur.

**Fini quand** — Marc rejoint depuis son téléphone sans compte, ferme
le navigateur, revient le lendemain et retrouve son personnage.

**Origine** — Planche « Inviter » (moments 1 et 2)

**État** — Migration `004_players.sql` (lien d'invitation, joueurs,
personnages). Le MJ ouvre une campagne depuis son accueil (liste et
import YAML minimal), page `/campagnes/:id` : le lien (un seul actif,
valable 7 jours, « Nouveau lien » révoque l'ancien, « Fermer le lien »),
le message Discord modifiable tiré de la vue joueurs (titre, accroche),
« Copier le lien et le message », et la table qui se remplit (relue
toutes les 5 s, en ligne / vu le…, retirer quelqu'un). Le serveur ne
garde que l'empreinte du code : l'appareil qui l'a créé le retient pour
le recopier, ailleurs il faut un nouveau lien (ceux qui ont rejoint
gardent leur place). Côté joueur, `/rejoindre/:code` : pseudo, puis
« Créer mon personnage » (place + brouillon de personnage) ou « Regarder
seulement » (spectateur) ; un navigateur qui a déjà sa place voit
« Reprendre ». Le jeton reste dans un cookie HttpOnly limité aux routes
de cette campagne, 400 jours ; `/partie/:id` montre la campagne et l'état
du personnage. Testé : rejoindre, revenir le lendemain avec le jeton,
mauvais jeton, jeton d'une autre campagne, lien régénéré / fermé /
expiré, spectateur, pseudo pris, aucune route MJ avec un jeton joueur.
Le joueur a sa socket en direct (`/api/play/{campaign}/live`, testée de
bout en bout : il reçoit `changed`, un joueur d'une autre table est
refusé) ; créer ou retirer un personnage touche `Topic::Character`.
**Reste** : que Romain l'essaie pour de vrai (téléphone de Marc, en
HTTPS) ; « reprendre » sur un autre appareil (lien de reprise donné par
le MJ) et reprendre un personnage d'une autre campagne (Hugo et Sef) ne
sont pas faits.

### `session/validate-characters` · doing — reste une vraie table

**Périmètre** — Le MJ voit sa table se remplir, relit chaque fiche,
valide ou renvoie avec un mot ; le joueur corrige et renvoie, le MJ ne
relit que la différence ; accroches secrètes tirées des histoires.

**Fini quand** — Les moments 3 à 6 de la planche « Inviter » sont
faisables dans l'app.

**Origine** — Planche « Inviter »

**État** — Côté MJ, livré et vérifié dans PCT 105 (`bun run lint`,
`bun run test`). Les règles vérifient chaque fiche
(`rules::check_character` : classe absente ou inconnue, caractéristique
inconnue, score hors classe, budget dépassé) et **signalent sans
bloquer**. La page de table (`/campagnes/:id/table`) suit le sujet live
`table` : chaque siège montre sprite, classe, présence et statut (« à
relire », « corrigé · à relire »…). La relecture d'une fiche affiche la
fiche, l'histoire, les points signalés et un mot proposé que le MJ
réécrit ; il **renvoie avec ce mot ou valide quand même** (décision
optimiste sur `updatedAt` : une fiche modifiée pendant la lecture
répond 409 et se recharge). À chaque décision la fiche est
photographiée (`reviewed_sheet`, migration 007, indépendante de 006) :
quand le joueur renvoie, le MJ ne lit que la différence (« Force :
18 → 17 »). Les accroches secrètes (table `secret_hooks`, jamais lue
par une route joueur, prouvé par le balayage marqué) sont **écrites à
la main par le MJ**, l'histoire du joueur sous les yeux, et nouées à des
scènes ou des fronts ; les proposer par l'IA reste à
`copilot/co-write-backstory`, la vérification des fiches par le co-MJ à
`copilot/check-character-sheets`. Depuis la fusion avec
`characters/build-character-creator`, le joueur enregistre et envoie sa
fiche : ses écritures laissent `reviewed_sheet` intact et appellent
`players::touch_character`, la table du MJ suit donc en direct ;
l'histoire s'affiche en trois réponses plus un paragraphe (une histoire
en texte simple est lue comme le paragraphe). **Reste** : jouer les
moments 3 à 6 avec de vrais joueurs sur téléphone.

### `session/schedule-sessions` · doing — reste un vrai rappel reçu sur le téléphone de Marc, par le vrai salon Discord de la table

**Périmètre** — Les joueurs donnent leurs disponibilités, le MJ choisit
la date ; rappel avant la session ; le salon ouvre à l'heure dite.

**Fini quand** — Le rappel arrive sur le téléphone de Marc et le mène
au salon d'un toucher.

**Origine** — Planches « Inviter » (moment 7) et « Entre deux » (moment 6)

**État** — Migration `029_schedule.sql`. Sur la page de table du MJ,
« Prochaine séance » : il propose des dates (jour, heure, durée), voit
pour chacune qui peut, qui ne peut pas, qui n'a pas répondu, et fixe
l'une d'elles (les autres se ferment). Côté joueur, onglet Jeu entre deux
séances : « Séance N : tu es libre quand ? », un toucher par date (« Je
peux » / « Je ne peux pas »), qui a déjà dit oui ; un spectateur voit les
dates sans répondre. Une fois la date fixée : la carte « Séance N · jeudi
10 octobre · 20 h 30 », l'heure d'ouverture du salon et « Ajouter à mon
agenda » (un fichier calendrier avec deux alarmes, la veille et une heure
avant, et le lien du salon). **Hypothèse retenue pour « le rappel arrive
sur le téléphone »** : la table de Romain vit déjà sur Discord, donc le
rappel part dans le salon Discord de la table par un webhook que le MJ
colle une fois (seule une adresse `https://discord.com/api/webhooks/…`
est acceptée ; elle reste côté MJ, qui n'en revoit que la fin). Discord
le pousse sur le téléphone de Marc ; le lien du message ouvre
`/partie/<campagne>`, dont l'onglet Jeu est le salon dès qu'il est
ouvert : un toucher. Une horloge du serveur (toutes les 30 s) envoie
l'annonce de la date, le rappel de la veille, celui d'une heure avant,
et **ouvre le salon tout seul un quart d'heure avant** (la même ouverture
que celle du MJ : campagne validée, nouvelle version des règles adoptée) ;
chaque étape n'est prise qu'une fois, même après un redémarrage, et un
serveur arrêté pendant la veille n'envoie au réveil que l'étape du
moment. Le MJ voit sous la date ce qui est parti et ce qui a échoué
(« pas parti : … »). Testé sur les deux mondes (proposition, réponses,
spectateur refusé, choix annoncé, calendrier, rappels un par un jusqu'au
salon ouvert, date close une fois jouée), plus le rattrapage, l'échec
du salon Discord et une campagne non validée qui garde son salon fermé ;
le webhook et le journal des rappels sont marqués dans le balayage des
routes joueur. **Reste** : brancher le vrai salon Discord de la table et
recevoir un vrai rappel sur le téléphone de Marc ; les notifications
Web Push de l'app (qui demandent l'app installée sur l'écran d'accueil
sous iOS) ne sont pas faites.

### `session/open-lobby` · doing — reste la soirée du jalon jouée pour de vrai

**Périmètre** — Le salon d'avant-session : présence en direct, test du
son, qui joue à distance ; le MJ lance quand la table est là.

**Fini quand** — Le MJ voit les trois joueurs arriver et lance la
session.

**Origine** — Planches « Mener » et « Lancer »

### `session/drive-scenes` · doing — reste la soirée du jalon jouée pour de vrai

**Pourquoi** — La boucle principale d'une soirée.

**Périmètre** — Le MJ montre une scène (texte lu, image), révèle un
indice ou une zone ; les joueurs proposent une carte d'action ou
« Autre… » ; la demande arrive au MJ en carte : valider, refuser avec
une raison, ou demander un test (difficulté en un geste) ; le résultat
revient au joueur ; le journal garde ce que le groupe sait.

**Fini quand** — Une scène de la démo se joue de la description au
résultat d'un test, chez le MJ et trois joueurs.

**Origine** — Planches « Jouer » et « Mener » · V1 `session/run-action.ts`

### `session/track-table-knowledge` · doing — reste la soirée du jalon jouée pour de vrai

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

### `session/collect-player-feedback` · doing — reste la soirée du jalon jouée pour de vrai

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

### `session/end-session` · doing — reste la soirée du jalon jouée pour de vrai

**Périmètre** — « Terminer la session » enregistre l'état ; le MJ écrit
ou colle le récapitulatif ; la chronique s'allonge d'une entrée.

**Fini quand** — La session suivante reprend exactement où la
précédente s'est arrêtée.

**Origine** — Planche « Mener » (fin) · V1 `continuity/`

### `session/write-recaps` · doing — reste un vrai récap relu et publié par Romain, lu par Marc le lendemain

**Périmètre** — Le co-MJ rédige le récapitulatif MJ, le « Précédemment… »
des joueurs et l'entrée de chronique ; le MJ relit et publie.

**Fini quand** — Tests `recap.test.ts` et `continuity.test.ts` du V1
portés ; Marc lit le « Précédemment… » le lendemain.

**Origine** — V1 `continuity/recap.ts` · planches « Mener » et « Entre deux »

**État** — Migration `028_recaps_and_launch.sql`. Le serveur photographie
le monde quand la séance démarre ; à la fin, il compare : scènes jouées,
indices trouvés, révélations désormais à portée, menaces qui avancent,
noms appris, plus les lignes du journal de la table (combats, butin,
promesses, dettes). « Terminer la séance » garde ce que le MJ a écrit et
rédige le reste à partir de ces faits (V1 `recap.test.ts` porté dans
`shared`) : le récap MJ (menaces et révélations comprises, et ce que la
suite demande), le « Précédemment… » et l'entrée de chronique (titre et
une ou deux lignes), ces deux-là sans aucune menace ni nom que la table
ne connaît pas. **Rien ne part aux joueurs avant que le MJ publie** :
après la séance, le panneau « Récapitulatifs » de l'écran du soir montre
les trois textes en brouillon, « Réécrire avec le co-MJ » (gabarit
`recap` v2, un appel compté), « Enregistrer », « Publier aux joueurs » ;
un texte joueur qui nomme une menace ou quelqu'un pas encore rencontré
est **signalé, jamais bloqué**, quel qu'en soit l'auteur. Publié, le
« Précédemment… » s'affiche sur le téléphone entre deux séances et dans
le salon, et la chronique (onglet Journal) gagne l'entrée de la séance ;
le récap du MJ ne sort jamais (balayage des routes joueur : brouillon et
récap marqués). Testé sur les deux mondes (V1 `continuity.test.ts`
porté : séance mesurée, brouillon factuel, réécriture, publication, la
séance suivante le retrouve et ne compte que le nouveau). **Reste** :
une vraie réécriture par OpenRouter relue par Romain ; que Marc la lise
le lendemain sur son téléphone. L'écran « Entre deux » complet (niveau,
fiche hors séance) est `player/play-between-sessions`, qui reprend la
chronique et le « Précédemment… » publiés tels quels.

### `session/pair-shared-screen` · doing — reste une vraie TV du salon jumelée, et une fenêtre partagée sur Discord pendant une soirée

**Périmètre** — La TV ouvre une page qui affiche un code et un QR ; le
MJ tape le code et la TV rejoint la session avec la projection « tous
les joueurs ». Ou le MJ partage cette page dans une fenêtre sur Discord.
Le MJ choisit ce que la TV peut montrer.

**Fini quand** — Une TV s'appaire en moins de 30 secondes et ne montre
rien de ce que la projection joueur refuse.

**Origine** — Planche « Lancer » (moments 1 à 3)

**État** — `/tv` sur la TV (ou un vieil ordinateur branché dessus)
affiche quatre lettres sans ambiguïté (ni 0/O ni 1/I/L) et le QR de la
page `/tv/jumeler?code=…`, où le MJ connecté choisit sa table. Le code
vit dix minutes et se renouvelle seul ; la TV redemande toutes les deux
secondes, donc le jumelage prend le temps de taper quatre lettres. Sur
l'écran de soirée du MJ, le bloc « Écran partagé » : saisir le code,
« Ouvrir la fenêtre TV » (le navigateur du MJ reçoit un écran jumelé
d'office, à partager dans Discord ; une seule fenêtre par campagne), la
liste des écrans avec « suit la partie » ou « hors ligne », « Oublier »,
et quatre interrupteurs de ce que la TV peut montrer (scène, carte,
groupe, grands moments), qui ne peuvent que retirer à la projection.
La TV garde son jeton (cookie HttpOnly sur `/api/tv`, empreinte seule
en base, migration `036`) et se reconnecte seule à la soirée suivante ;
oubliée, elle revient au code. Un écran n'est jamais compté comme un
joueur dans la présence. Le jeton d'écran n'ouvre ni route MJ ni route
joueur, et la session MJ n'ouvre aucune route d'écran : la fenêtre
partagée vit dans le navigateur du MJ sans jamais rien voir de MJ. Tests :
le jumelage de bout en bout (code faux, code expiré, code déjà pris par
un autre MJ, oubli), la fenêtre, la présence et le signal en direct sur
une vraie socket, le balayage des fuites des routes d'écran sur la table
marquée, les routes MJ dans le balayage `require_gm`, et les écrans en
Vitest.

---

## Épic `player`

L'écran du joueur, d'après la planche « Jouer · la soirée de Marc » :
un bandeau de statut, un sujet, une action principale ; quatre onglets
Jeu, Carte, Perso, Journal.

### `player/play-scene` · doing — reste la soirée du jalon jouée pour de vrai

**Périmètre** — Onglet Jeu : la scène (lieu, texte lu, image), le
mini-lecteur de musique, les indices reçus en cartes, la main de cartes
d'action et « Autre… », le dé à lancer quand le MJ demande un test, le
résultat.

**Fini quand** — Les moments de scène de la planche « Jouer » sont
faisables dans l'app sur un iPhone et un Android.

**Origine** — Planche « Jouer » · V1 écran joueur

### `player/read-the-rules` · doing — reste le premier combat joué (phase 3)

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

**État** — La page `/partie/:id/regles` (bouton « Les règles » sur
l'accueil du joueur) est entièrement tirée du système de règles par la
projection (`campaigns/projection/rules.rs`, `GET /api/play/{id}/rules`) :
le dé et le modificateur, les difficultés, les quatre résultats (faces
naturelles, XP, dégâts ×2), l'attaque et la CA, le tour et le coût de
chaque action, la recharge et les durées, les états, 0 PV, la
progression, la grille. Rien n'y est écrit à la main hors des phrases de
liaison ; ni adversaires, ni paliers, ni notes du MJ (test). Avec une
fiche qui a une classe : ce qu'il faut au dé pour chaque caractéristique
contre chaque difficulté, chaque carte avec son coût en actions, sa
recharge et son jet d'attaque détaillé (`action::attack_modifiers`,
extrait du moteur), et quatre jets d'exemple, un par résultat, faits par
le moteur lui-même avec la face imposée. Le composant `RollDetail`
affiche un `RollBreakdown` (le type d'un vrai jet) : face, chaque bonus
et sa source, total, cible, résultat et pourquoi (« 9 contre Moyen
(10) : il manquait 1 »). « Ce qui change » : diff entre deux versions
(`rules::changes`, testé sur des variantes des Corsaires) ; la version
lue par chaque joueur est dans `rules_seen` (migration 009, posée à
l'arrivée à la table), le joueur la fait avancer avec « J'ai lu ce qui
change ». Vérifié sur les deux mondes (`back/tests/rules_test.rs`),
balayage des routes joueur vert.
Limites honnêtes : il n'existe encore qu'une version (v1) de chaque
système, donc personne ne verra de liste de changements avant
`campaign/edit-rule-system` ; les sessions n'existent pas encore, donc
« jamais en pleine partie » se réduit à : la liste s'affiche dès que les
règles changent et reste jusqu'à lecture — à masquer pendant une session
ouverte quand `session/open-lobby` existera. Les nombres du joueur sont
ceux du niveau 1 de sa fiche (pas encore son état vivant). Attend la
phase 3 : un vrai jet en séance affiché par `RollDetail`
(`ui/roll-faceted-dice`, `player/fight-turn`) — c'est là que le « fini
quand » se vérifiera.

### `player/buy-and-trade` · doing — reste le marché de Kerjean joué pour de vrai, et la monnaie du Brasier à trancher par Romain

**Périmètre** — L'or ; une boutique ouverte par le MJ (prix, stock,
objet « sous le comptoir » révélé par un jet ou une discussion) ;
marchander par un jet ; partager le butin.

**Fini quand** (hypothèse, écrite d'après Kerjean et le Brasier) — Au
marché noir de Kerjean, le MJ ouvre la boutique de Dents-de-Fer en un
geste depuis l'histoire ; un joueur achète depuis sa bourse, marchande
une fois (le serveur lance Charisme contre 10 : moitié prix sur un
achat au choix arrondi en faveur du marchand, +2 PO sur tous les prix
sur un 1, la boussole sort sur un 20) ; la boussole reste invisible
aux téléphones tant que le MJ ne la sort pas ; deux joueurs se passent
de l'or et un objet. Une table Brasier fait de même une fois que le MJ
a donné une monnaie à ses règles.

**Origine** — Marché noir de Kerjean, Corsaires acte 1

**État** — Livré et testé sur les deux mondes (`back/tests/trade_test.rs`,
moteur `rules::trade`, vitest `features/play/trade`), balayages MJ et
joueur verts (une ligne cachée marquée ne sort jamais). Le MJ, sur son
écran en direct (panneau Boutiques), ouvre la boutique d'un PNJ qui vend
— ses prix, ce qu'il cache dans son inventaire « sous le comptoir » à la
valeur de l'objet, le test « Marchander » de sa scène — ou une boutique
vide ; il édite le comptoir (objet des règles, de l'histoire ou qu'il
nomme ; prix, stock, caché), les termes du marchandage, ouvre et ferme,
sort une ligne cachée (le journal le dit), voit qui a marchandé.
Migration 033. Côté joueur, onglet Jeu, en séance ou entre deux : la
boutique ouverte, sa bourse, « Acheter » (une unité, payée dans la
monnaie des règles, posée dans le sac par `players::play`, journalisée
« par le joueur »), « Marchander » une fois par personnage et par
boutique (jet du serveur, XP du résultat, détail du jet affiché), puis
« Acheter à 8 PO » tant que le rabais n'est pas utilisé. Onglet
Personnage : « Donner à un compagnon » (un objet du sac ou de l'or) ;
la ligne du journal nomme celui qui reçoit et apparaît dans sa fin de
soirée. La monnaie est la première ressource des règles, la même que
celle du butin.
Hypothèses et limites : le 20 de Kerjean sort la boussole « pour
15 PO » ; ici elle sort au prix du comptoir (25) et le rabais gagné
peut la mettre à 13. Le Brasier ne déclare toujours aucune monnaie
(« Crédits / monnaie d'échange : ____ » sur la fiche du Cure-Dent) :
le panneau le dit au MJ et le test l'ajoute par l'éditeur de règles
(« Crédits », départ 0) — à trancher par Romain. Pas de vente au
marchand (les plaques de chitine Vorr « intéresseront un marchand ») ni
de partage automatique d'un butin d'or entre tous. Jamais joué.

### `player/explore-map` · doing — reste la soirée du jalon jouée pour de vrai

**Périmètre** — Onglet Carte : carte révélée, pion glissé au doigt, cases
atteignables surlignées, trajet validé par le serveur, brouillard,
fantôme quand on est invisible.

**Fini quand** — Marc déplace Borin au doigt sans jamais viser ni
zoomer.

**Origine** — Planche « Jouer » (carte)

### `player/fight-turn` · doing — reste la soirée du jalon jouée pour de vrai

**Périmètre** — « À toi, Borin ! » ; ordre du tour, carte centrée sur la
cible, cœurs et gemmes, main de cartes en éventail, grappe arcade
(gros bouton, objet, fin du tour), dé coloré, dégâts.

**Fini quand** — Le combat de la démo se joue du premier tour au butin.

**Origine** — Planche « Jouer » (combat)

### `player/read-sheet-and-journal` · doing — reste l'essai sur un vrai téléphone

**Périmètre** — Onglet Perso (fiche, cartes, sac, équiper) et onglet
Journal (ce que le groupe sait).

**Origine** — Planche « Jouer » (onglets) · planches Joueur

**État** — Livré et vérifié dans PCT 105 (`bun run lint`, `bun run
test`). La page du joueur (`/partie/:id`) a deux onglets en bas,
**Personnage** et **Journal** ; les onglets Jeu et Carte de la planche
viendront avec la soirée (phase 3), sans bouton d'ici là. Un spectateur
n'a que le Journal. Personnage : tant que la fiche n'est pas validée,
la carte d'avant (statut, mot du MJ, créateur) ; une fois en jeu, la
fiche du même état que le MJ ajuste (`gm/adjust-sheets-fast`) — sprite,
niveau et barre d'XP, cœurs et PV, armure, attaque, initiative,
caractéristiques et modificateurs, cartes de la classe (celles d'un
niveau à venir grisées, « au niveau 3 »), bourse, « Sur toi » et « Ton
sac ». Tout vient de la projection joueur ; la page suit le canal live
(sujet de son personnage, puis `world`/`story` pour le Journal). Le seul
geste du joueur est **équiper / ranger** (`POST
/api/play/{id}/character/equip`, verrou de campagne, journalisé « par
le joueur » dans l'historique du MJ) ; les règles des deux mondes ne
donnent aucun effet à l'équipement, et l'écran le dit au lieu de le
laisser croire. Les objets s'affichent en liste nommée : les règles
n'ont pas encore d'art d'objet (`media/draw-pixel-art-assets`).
Journal : ce que la projection joueur contient déjà — ce qu'on a dit
aux joueurs, la scène en cours, les indices trouvés, les PNJ rencontrés
— et « Le groupe n'a encore rien découvert » sinon. Aujourd'hui aucun
écran ne révèle d'indice ni ne change de scène : le Journal se remplira
avec `session/drive-scenes` et `session/track-table-knowledge`
(promesses, dettes, décisions de règle, « ce soir »), rien n'y est
simulé. Pas encore essayé sur un vrai téléphone ni en vraie session.

### `player/receive-rewards` · doing — reste la soirée du jalon jouée pour de vrai

**Périmètre** — Toasts qui tombent et s'empilent, butin, compétence
débloquée, niveau.

**Origine** — Planches « Notifications » et « Objets et butin »

### `player/play-between-sessions` · doing — reste les dates (moment 6, avec `session/schedule-sessions`) et un vrai entre-deux-séances sur un téléphone

**Périmètre** — Monter de niveau à la fin de la soirée, lire le récap
et la chronique, consulter sa fiche hors session, donner ses dates.

**Fini quand** — Les six moments de la planche « Entre deux » sont
faisables dans l'app.

**Origine** — Planche « Entre deux »

**État** — Moments 1 à 5 livrés et testés sur les deux mondes
(`back/tests/between_test.rs`, vitest `features/play/between`).
Hors séance, l'onglet Jeu devient l'entre-deux : fin de la séance N
(durée jouée, scène où elle s'est arrêtée), l'XP gagnée ce soir-là et
le niveau atteint, ce que le personnage a reçu (le butin donné par le
MJ porte désormais le personnage dans le journal, migration 032), puis
« Précédemment… » une fois publié — sinon « le MJ relit » —, ce que la
table a appris ce soir-là, ce qui reste ouvert (promesses et dettes), et
les chemins vers la chronique (onglet Journal : une entrée par séance,
la dernière marquée) et la fiche. Monter de niveau : dans les deux
mondes, un niveau ne donne pas de PV (10 fixes) mais des points
d'amélioration (un par 5 XP) et des cartes ; l'onglet Personnage ouvre
alors « Borin · niveau N » : choisir une caractéristique, +1, un point à
la fois, et les cartes que le niveau vient d'ouvrir. Le serveur dépense
le point (refusé pendant une séance en cours : les nombres ne bougent
pas en pleine partie), les points dépensés sont stockés
(`character_play.upgrades`) et appliqués par le moteur partagé à chaque
calcul (jets, combats, fiche du MJ, qui lit « place un point en Force »
dans l'historique). Points restants = gagnés − dépensés, jamais
négatifs si le MJ reprend de l'XP. Fiche hors séance : toucher une carte
montre ce qu'elle fait avec les nombres du serveur (toucher, dégâts,
recharge, portée). Projection par liste blanche
(`campaigns/projection/between.rs`) : ni récap MJ, ni lignes MJ, ni
historique des ajustements (seules des sommes d'XP du joueur en sont
tirées), balayage joueur vert. « Précédemment… » passe par un seul
point (`Session::published_previously`) que `session/write-recaps`
réglera pour les brouillons.
Hypothèses et limites : les PV au dé ou à la moyenne n'existent dans
aucun des deux systèmes ; ils viendront avec `engine/level-up` pour un
système à dés de vie. La chronique n'a pas encore les vignettes des
images montrées en jeu. Le moment 6 (donner ses dates, le rappel qui
mène au salon) est `session/schedule-sessions` : aucun bouton ici d'ici
là. Jamais essayé entre deux vraies séances.

### `player/face-death` · doing — reste une mort vécue sur un vrai téléphone

**Périmètre** — Jets contre la mort sur le téléphone, derniers mots,
puis la suite : regarder, créer un nouveau personnage, ou attendre une
accroche du MJ.

**Origine** — Planche « Mourir »

**État** — Livré et vérifié (Vitest, tests serveur). Onglet Carte : à
terre, le joueur voit « Tu es à terre » et ses cases de réussites et
d'échecs ; à son tour, la main et la grappe arcade laissent la place au
seul « Lancer le jet contre la mort » ; trois échecs : « Le MJ regarde
ce qui se passe… ». Un allié, à son tour, a « Stabiliser Borin ».
L'ordre du tour montre les jets de chaque mourant. Après la mort, le
joueur garde sa place : bandeau « Ton personnage est tombé », le
personnage en gris, ses derniers mots (une phrase, une seule fois, lus
par toute la table dans le journal), puis « Regarder ce soir »,
« Créer un nouveau personnage » (définitif : un brouillon qui arrive
avec l'XP du mort une fois validé par le MJ) ou « Attendre une
accroche ». Le MJ retrouve les morts dans « Tombés », sous les fiches,
avec leurs derniers mots et le choix du joueur. Hypothèse posée : la TV
(silence, portrait) et « Léguer ses objets » de la planche ne sont pas
dans ce lot (`tv/show-evening`, `player/buy-and-trade`).

### `player/play-on-desktop` · doing — reste une soirée jouée par un joueur sur son ordinateur

**Périmètre** — La même partie dépliée sur un grand écran : scène, main,
carte et combat côte à côte ; raccourcis clavier (chiffres pour les
cartes, espace pour le dé).

**Fini quand** — Un joueur suit une soirée entière depuis son
ordinateur sans jamais changer d'onglet, et joue ses cartes, lance ses
dés et mène ses tours de combat au clavier, sur une table Corsaires
comme sur une table Brasier.

**Origine** — Planche « Jouer sur ordinateur »

**État** — Livré et testé (Vitest), sans changement serveur : c'est la
même projection joueur, mise en page autrement. Sur un écran large
avec une souris (1100 px et plus), la page de partie quitte ses onglets
: la fiche à gauche (cœurs, gemmes, cartes, sac, ou le personnage en
cours de création), la scène, la musique, les demandes et la main au
centre, la carte, le combat et le journal à droite ; chaque partie
n'est chargée qu'une fois. Clavier : 1 à 9 choisissent les cartes de
la main dans l'ordre, 0 « Autre… », Échap repose la carte, Entrée
envoie ce qui est écrit (Maj+Entrée : une ligne), Espace lance le dé
que le MJ a demandé (le plus ancien) ; en combat, à mon tour, les
chiffres prennent les cartes de combat et Entrée joue. Les touches se
taisent quand on écrit dans un champ, avec une touche de modification,
et Espace ou Entrée ne doublent jamais un bouton qui a le focus. La
carte se fait glisser à la souris. Une tablette garde la mise en page
du téléphone. Les deux mondes ont six caractéristiques plus les cartes
de classe : au-delà de neuf cartes, les suivantes n'ont pas de touche
(on les clique). Pas encore : le panneau « le groupe » de la planche
(les PV des autres joueurs hors combat ne sont pas dans la projection
joueur). Jamais essayé par un vrai joueur, ni vu rendu dans un vrai
navigateur.

---

## Épic `gm`

### `gm/run-live-screen` · doing — reste la soirée du jalon jouée pour de vrai

**Pourquoi** — Le MJ mène toute la soirée d'un seul écran.

**Périmètre** — L'écran MJ en direct sur ordinateur : une action
principale par moment, scène et sorties, carte complète avec ce qui est
caché, demandes des joueurs en cartes, journal qui montre aussi le
caché, la table et ses présences, musique.

**Fini quand** — Romain mène une soirée à six joueurs sans quitter cet
écran (les planches en montrent trois : tout doit tenir à six).

**Origine** — Planche « Mener · la soirée de Marc côté MJ »

### `gm/run-combat` · doing — reste la soirée du jalon jouée pour de vrai

**Périmètre** — Lancer une rencontre (la carte de combat s'ouvre
partout), initiative, jouer les adversaires, poser et retirer des
états, valider le butin.

**Fini quand** — Un combat à six joueurs contre six adversaires (la
bagarre du quai) se mène côté MJ jusqu'au butin.

**Origine** — Planche « Mener » (combat)

### `gm/balance-spotlight` · doing — reste la soirée du jalon jouée pour de vrai

**Pourquoi** — Divertir six joueurs à la fois est ce qui épuise le MJ.

**Périmètre** — Pour chaque joueur : depuis quand il n'a rien fait ni
demandé, ses demandes en attente, les accroches de son histoire pas
encore jouées ; le co-MJ propose une occasion de lui donner la main
dans la scène en cours.

**Fini quand** — Sur une soirée de deux heures, le MJ est averti dès
qu'un des six joueurs reste vingt minutes sans moment à lui.

**Origine** — Romain, 4 octobre 2026 · `MEMORY.md` §6

### `gm/adjust-sheets-fast` · doing — reste une vraie table

**Périmètre** — Depuis l'écran MJ, en un geste : +1 XP, retirer ou
rendre des PV, donner un objet, de l'or ; la fiche du joueur change en
direct, et chaque modification va dans l'historique de la session.

**Origine** — `dnd-save/DnD-16-05-2026/prompt_plateforme_fiches.md`

**État** — Livré et vérifié dans PCT 105 (`bun run lint`, `bun run
test`), sur les deux mondes. La fiche écrite et relue (`sheet`,
`reviewed_sheet`) ne bouge pas : l'état en jeu est à part, tenu par le
serveur (migration 008, `character_play`) — XP totale, **PV perdus**
(pas restants : si le maximum monte, les blessures restent), ressources
des règles, sac. Une fiche validée sans ligne joue depuis son état de
départ (les PV pleins, l'or de départ des règles, les objets de sa
classe). PV max, niveau, barre d'XP, points d'amélioration, CA et
cartes débloquées sont **calculés par le moteur de règles**, jamais
saisis ni stockés. Sur la page de table, « Fiches en jeu » (dès qu'un
personnage est validé) montre une carte par personnage — cœurs, XP,
bourse, sac — avec −1 PV, +1 PV, +1 XP en un geste, et « Plus… » pour
un montant, l'or, donner un objet des règles ou un objet nommé par le
MJ, reprendre un objet. Chaque geste prend le verrou de la campagne,
écrit, journalise et appelle `touch_character` dans la même transaction
: la fiche du joueur (`me` → `play`) et la page du MJ suivent en direct.
L'historique (`play_adjustments`, append-only : qui, quoi, avant,
après, quand) est **celui de la campagne** : il n'existe pas encore
d'entité session (`session/end-session`), une session en sera une
tranche de temps. Il reste côté MJ : la note MJ d'un objet des règles
n'est jamais montrée, le balayage joueur le prouve avec un marqueur
dans l'historique. Limites : l'« or » est la ressource que les règles
déclarent — les Corsaires ont leurs pièces d'or, **le Brasier n'en
déclare aucune**, donc pas de bouton d'or sur une table Brasier tant
que Romain n'en ajoute pas une à ses règles ; PV à 0 n'applique pas
encore l'état « Inconscient » (avec `gm/run-combat`) ; dépenser un
point d'amélioration se fait entre deux séances (`player/play-between-sessions`) ; l'écran
MJ en direct complet est `gm/run-live-screen`. Jamais essayé à une
vraie table.

### `gm/launch-session` · doing — reste un vrai lancement à la table, et la TV quand elle existera

**Périmètre** — Le lancement : salon, TV, « Précédemment… » lu ligne à
ligne, première scène.

**Origine** — Planche « Lancer » (moments 4 et 5)

**État** — Dans le salon, l'écran du soir du MJ montre « Avant de
lancer » : le récap de la séance précédente publié ou non (sinon rien
ne sera lu), combien de joueurs sont là et avec le son. « Lancer la
partie » démarre la séance ; si le dernier « Précédemment… » est publié,
sa lecture commence : la première phrase part sur tous les téléphones,
le MJ voit le texte entier, ce qui est déjà lu, son propre récap « pour
vous seul », et envoie « Phrase suivante » à son rythme (jamais au-delà
de la dernière). Le texte est coupé en phrases par le serveur (retours à
la ligne et fins de phrase, guillemets français compris). Sur le
téléphone, l'onglet Jeu affiche les phrases reçues, la dernière en clair,
et « Le MJ lit la suite… » ; la scène attend. « Envoyer la première
scène : … » (là où la table s'était arrêtée, sinon la première scène de
la campagne) termine la lecture partout — comme n'importe quelle scène
montrée. Les phrases pas encore lues ne quittent pas le serveur ; la
lecture passe par la projection joueur, donc la TV la recevra sans code
neuf. Testé sur les deux mondes (lecture phrase par phrase, plafond,
fin de lecture par la scène, rien à lire sans récap publié). **Reste** :
le jumelage de la TV (`session/pair-shared-screen`) et son affichage
(`tv/show-evening`) — aucun bouton TV n'est montré d'ici là ; la lecture
par la voix du co-MJ n'est pas faite ; un vrai lancement à la table.

### `gm/run-on-tablet` · doing — reste une soirée entière menée depuis un iPad

**Périmètre** — L'écran MJ sur tablette : rail de grosses cibles à la
place des onglets, demandes au pouce, carte au doigt (un doigt peint le
brouillard, deux déplacent, pincer zoome), propositions du co-MJ en
trois grosses touches.

**Fini quand** — Romain mène une soirée entière depuis un iPad.

**Origine** — Planche « Tablette »

**État** — Livré et testé (Vitest), sans changement serveur : chaque
geste passe par les routes de l'écran MJ existantes. L'écran de soirée
passe en mode tablette sur un appareil tactile de 768 px et plus, ou
avec `?ecran=tablette` (`?ecran=ordinateur` force l'ordinateur ; un
bouton de l'en-tête bascule). Un rail de cinq grosses cibles remplace
les trois colonnes : Scène (les demandes en attente, la scène, les
images), Table, Carte (la carte et le combat), Journal (avec la fin de
soirée et les retours) et Co-MJ, qui glisse en tiroir par-dessus sans
quitter la section. Un point sur une cible dit qu'une chose y attend
(une demande, un joueur oublié, le tour d'un adversaire, un brouillon
du co-MJ). À droite, dès 1000 px, les places en grand : qui est là,
depuis quand il n'a rien fait, qui demande ; un appui ouvre la Table,
où seul « Donner la main » compte un moment (un pouce égaré sur une
tablette tenue en main ne doit rien écrire). Tous les boutons prennent la taille d'un doigt. Une demande se
juge au pouce : la caractéristique en grosses touches, puis une touche
par difficulté des règles (quatre aux Corsaires) envoie le test ; « Oui,
sans jet » ; « Non » part avec « Non, rien ici. » si le MJ n'a rien
écrit (le serveur exige un mot). La carte : un doigt touche ou peint le
brouillard (outils Révéler, Cacher), un second doigt annule le trait,
deux doigts la font glisser, pincer zoome, rien ne part avant le
relâcher. Au tour d'un adversaire, trois grosses touches : « Valider »
la proposition du co-MJ (ou la demander), « Il fuit », « Il passe son
tour » — la planche disait « Changer », mais le MJ ne peut pas encore
choisir à la main l'action d'un adversaire, donc la touche dit ce
qu'elle fait. Un brouillon du co-MJ se lit en entier, avec « Montrer »,
« Modifier », « Écarter ». Pas de micro : parler au co-MJ est
`copilot/listen-by-voice` ; le tiroir est l'endroit où il se branchera.
Jamais essayé sur un vrai iPad : le zoom au pincement, en particulier,
n'a été vérifié que par sa logique, pas sous les doigts.

---

## Épic `copilot`

Tout ce que le co-MJ produit est un brouillon que le MJ modifie ou
valide (`MEMORY.md` §3). Spécification : V1 `copilot/` et ses tests.

### `copilot/draft-narration` · doing — reste la soirée du jalon jouée pour de vrai

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

### `copilot/propose-adversary-turns` · doing — reste la soirée du jalon jouée pour de vrai

**Périmètre** — En combat, le co-MJ propose le tour de chaque adversaire
(cible, action, jet déjà résolu) ; le MJ valide, change ou fait fuir.

**Origine** — Planches « Mener » et « Tablette » (moment 4)

### `copilot/check-character-sheets` · doing — reste un vrai mot proposé avec la clé OpenRouter sur une vraie fiche

**Périmètre** — Le co-MJ vérifie une fiche envoyée contre les limites
de création et propose un mot au joueur ; le MJ décide.

**État** — Les vérifications restent celles du système de règles
(`session/validate-characters`, rien n'est bloqué). Sur une fiche
envoyée, « Le co-MJ propose un mot » remplit le mot au joueur à partir
de ces vérifications et de la fiche (gabarit `sheet-note.v1`, appel
compté `sheet.note`) : tutoiement, ce qui dépasse et quoi changer, le
rappel que le MJ a le dernier mot. Rien ne part : le MJ modifie le mot,
puis renvoie ou valide quand même.

**Origine** — Planches « Inviter » (moments 3 et 4) et « Créer » (moment 6)

### `copilot/co-write-backstory` · doing — reste une vraie histoire et de vraies accroches avec la clé OpenRouter

**Périmètre** — Le co-MJ pose des questions au joueur pour écrire son
histoire, sans rien inventer à sa place ; il en tire des accroches
secrètes pour le MJ.

**État** — Côté joueur, à l'étape « Son histoire », « Écrire avec le
co-MJ » fait un paragraphe des trois réponses (gabarit `backstory.v1`,
appel compté `backstory.write` sur le budget de la campagne), que le
joueur garde ou modifie avant l'envoi. Un nom propre absent de ses
réponses fait refaire le paragraphe une fois, puis il est refusé. Côté
MJ, dans « Accroches secrètes », le co-MJ propose une à trois accroches
tirées de l'histoire (gabarit `hooks.v1`, appel `hooks.propose`), liées
seulement à des scènes et fronts qui existent (les liens inventés sont
retirés et comptés) ; le MJ garde, retouche ou écarte chacune, rien
n'est enregistré sans lui. Écart : les trois questions sont fixes (pas
encore adaptées à la campagne).

**Origine** — Planche « Créer » (moment 7)

### `copilot/listen-by-voice` · doing — reste une vraie dictée transcrite avec la clé OpenRouter, sur une tablette en HTTPS

**Périmètre** — Dicter au co-MJ (tablette surtout) ; transcription et
proposition à valider.

**Hypothèse retenue** — C'est le MJ qui parle au co-MJ, micro tenu
(« touche pour parler », planche « Tablette », moment 5) ; le co-MJ
n'écoute pas la table en continu : la voix des joueurs reste sur
Discord (`MEMORY.md` §1), et écouter six joueurs en permanence coûterait
cher et poserait la question de leur accord. Ce qu'il entend remplace ce
que le MJ aurait tapé : la réponse est un brouillon comme les autres.

**Fini quand** —
- Sur l'écran MJ en direct, le MJ touche « Parler au co-MJ », parle,
  touche à nouveau : ce qu'il a dit s'affiche (« Vous avez dit : … »)
  et le co-MJ répond avec le type choisi (Décrire, Faire parler un PNJ,
  Conséquence, Et ensuite ?, Libre par défaut).
- La réponse est un brouillon ordinaire : le MJ le modifie, le montre
  ou l'écarte ; ni ses mots ni le brouillon n'atteignent un téléphone
  sans ce geste. « Changer » remet ce qui a été entendu dans le champ
  texte pour le corriger et redemander.
- La transcription passe par le trait de fournisseur (faux fournisseur
  en test, OpenRouter en vrai), avec un gabarit versionné qui donne au
  modèle les noms propres de la campagne (Vaubernier, LUMEN, le
  Cure-Dent…) ; elle est comptée sur le budget IA, refusée avant
  l'envoi si le budget ne suffit pas ou si la partie n'est pas lancée.
- Un silence ne coûte qu'une écoute (« Le co-MJ n'a rien entendu ») ;
  une minute au plus par dictée.
- Testé sur les deux mondes, Corsaires et Brasier.

**État** — Livré et vérifié (`cargo test`, Vitest). Le bouton micro est
dans le panneau co-MJ de l'écran en direct, grand pour le doigt ; le
tiroir de la tablette qui l'accueillera est à `gm/run-on-tablet`. Le
navigateur enregistre et envoie un WAV 16 kHz (même format sur Chrome et
Safari) ; le serveur l'écrit (gabarit `transcribe.v1`, appel compté
`copilot.voice`) puis demande au co-MJ (`copilot.<type>`) : deux appels
par dictée. Sans micro possible (page en HTTP hors de l'ordinateur
local), le panneau le dit au lieu d'afficher un bouton. Écarts : aucune
transcription réelle n'a encore tourné (modèle par défaut
`google/gemini-2.5-flash`, `OPENROUTER_AUDIO_MODEL`) ; l'autorisation
micro de l'app iOS (`src-tauri/Info.ios.plist`) n'a pas été essayée dans
le simulateur, et Android (`RECORD_AUDIO`, manifeste généré par
`tauri android init`) comme le bureau macOS (sa propre phrase
d'autorisation micro) restent à faire ; l'enregistrement sur un vrai
iPad n'est pas mesuré ; un budget qui couvre l'écoute mais pas la
réponse paie l'écoute et perd ce qui a été dit ; la proposition « Faire fuir le gobelin » de la planche
n'a pas de bouton : les gestes proposés restent ceux du co-MJ écrit
(indice, menace, scène, PNJ).

**Origine** — Planche « Tablette » (moment 5)

---

## Épic `tv`

### `tv/show-evening` · doing — reste une soirée à six suivie sur une vraie TV, et les niveaux quand `engine/level-up` existera

**Périmètre** — L'écran partagé : un point focal à la fois (histoire, dé,
carte ou butin), lisible d'un canapé ; fil d'une ligne en bas ; grands
moments (dés, coups, butin, niveaux, révélations) ; rien de secret.

**Fini quand** — La TV suit une soirée à six joueurs comme sur la
planche, sans action du MJ autre que l'appairage.

**Origine** — Planches « Écran TV » et « TV · la soirée côté TV »

**État** — Une seule réponse serveur, `projection::screen`, construite
depuis la vue des joueurs et la grille du spectateur, puis réduite par
les interrupteurs du MJ : titre, session, « Précédemment… », musique, le
groupe (personnage, pseudo, apparence, présent ou non, PV — jamais un
spectateur), la scène telle que les joueurs la voient, la carte telle
qu'un spectateur la voit (brouillard, pions cachés et invisibles exclus),
les lignes partagées du journal de la session, les tests lancés réduits
à ce que la ligne du journal dit déjà (qui, quelle caractéristique, la
difficulté, les dés, l'issue), et « Ce soir » une fois la session close.
La TV choisit seule son point focal : salon (les sièges, qui est là),
« Précédemment… » phrase par phrase avant la première scène, la scène
avec son image approuvée, la carte et le groupe en cœurs, le combat avec
l'ordre du tour et la dernière ligne du combat ; par-dessus, un grand
moment à la fois pendant six secondes (dé coloré qui roule puis l'issue,
indice en carte qui se retourne, butin, objectif atteint, rencontre),
jamais rejoué après une reconnexion de plus de 30 s ; en combat, chaque
coup porté s'affiche en grand chiffre rouge deux secondes et demie
(« Hors de combat ! » s'il fait tomber), les dégâts lancés pour un
adversaire dont les PV restent cachés, une TV ouverte en plein combat
ne rejouant aucun coup déjà porté. Scène 1920 × 1080
mise à l'échelle de la fenêtre, lecteur YouTube visible, mouvement réduit
respecté. Testé : sur les deux mondes (fixture Corsaires/Kerbrume au
balayage, Brasier à l'API) ; le choix du point focal et des moments en
Vitest. **Écarts** : les niveaux n'ont pas de grand moment (le passage de
niveau est `engine/level-up`, d'un autre lot ; le jour où il écrit une
ligne partagée au journal, l'ajouter aux genres de `tv/focus.ts`) ; les coups s'affichent en plein
écran par-dessus la carte, pas sur le pion touché comme sur la planche ;
jamais essayé à six sur une vraie TV.

---

## Épic `characters`

Les personnages sont des sprites pixel art en couleur, façon Terraria /
Starbound (voir `MEMORY.md` §2). Dans le design actuel, ils sont fixes
et vus de profil ; dans l'app, chaque joueur fabrique le sien.

### `characters/render-layered-sprite` · doing — reste à regarder les douze personnages sur la page de référence

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

**État** — Fait : le format de description (`shared/src/sprite/`,
`CharacterLook` en YAML ou JSON), les packs `marins-1718` et
`equipage-spatial` (`content/sprites/<pack>/pack.yaml` : calques en
grilles de caractères, palettes peau / cheveux / tissu, une variante par
corps svelte ou robuste), le rendu en Rust (ordre des calques, palette,
ombrage trois tons du prototype, contour d'un pixel autour de la
silhouette, jamais sur le visage, ombre portée) et l'allure des six
personnages types et des adversaires de chaque monde
(`content/sprites/looks/<monde>.yaml`, mêmes identifiants que les
campagnes). Le serveur dessine seul : `GET /api/sprites/render.png`
renvoie le PNG d'une description (mis en cache par empreinte), et
`<Sprite look scale facing effects />` l'agrandit en pixels nets ; le
téléphone, l'écran MJ et la TV chargent donc la même image. Les effets
d'état (poison, étourdi, endormi, invisible, feu, entravé, effrayé,
béni, KO, cible) sont posés par-dessus en CSS, et chaque état du
système de règles dit lequel (`conditions[].visual`). Les Vorr sont des
humanoïdes en carapace, casque fermé, recolorés chitine et ambre : le
pack ne dessine que des humains. Images de référence :
`shared/tests/golden/<monde>/planche.png`. Reste pour Romain : regarder
les douze personnages et les adversaires sur `/reference` et dire ce qui
cloche ; les vues de face et de dos sont arrivées avec
`characters/walk-in-four-directions`.

### `characters/build-character-creator` · doing — reste l'essai sur un vrai téléphone

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

**État** — Vérifié dans PCT 105 (`bun run lint`, `bun run test`). Le
joueur arrive par son lien et ouvre le créateur
(`/partie/:id/personnage`) : peuple (quand les règles en ont), corps,
tenue, couleurs et nom, classe, caractéristiques (budget dépassé
signalé, jamais bloquant), histoire en trois questions plus un
paragraphe, relecture et envoi au MJ. L'aperçu est le sprite dessiné par
le serveur depuis la description en couches (`look`, jamais une image),
avec un bouton « Au hasard ». Le serveur tient le brouillon
(`PUT /api/play/:c/character`, nettoyé par `CharacterSheet::cleaned`)
et l'envoi (`POST …/character/submit` : 400 `CHARACTER_INCOMPLETE`,
409 `CHARACTER_LOCKED` une fois envoyé) ; chaque écriture prévient la
table du MJ (`touch_character`), qui relit avec
`session/validate-characters`. L'accueil joueur montre le personnage
dessiné et propose « Créer / Reprendre / Corriger » tant que la fiche
est en brouillon ou renvoyée. Un test suit Marc sur les huit moments.
**Limites** : aucun des deux mondes témoins ne définit encore de
peuples, l'étape « peuple » n'apparaît donc qu'avec un système qui en a ;
le personnage est sur la carte depuis
`characters/walk-in-four-directions`, la TV attend `tv/show-evening` ; le même
créateur côté MJ pour les PNJ et monstres reste à faire. **Reste** : un
joueur crée son personnage sur un vrai téléphone en moins de deux
minutes.

### `characters/walk-in-four-directions` · doing — reste un vrai déplacement vu sur plusieurs téléphones, la TV, et la relecture des vues de face et de dos par Romain

**Pourquoi** — Sur la carte, un personnage qui se tourne vers là où il
va rend le déplacement lisible et vivant.

**Périmètre** — Chaque couche existe en 4 directions (face, dos,
gauche, droite) ; animations repos, marche, attaque, touché par
direction ; le pion se tourne vers sa case d'arrivée ou sa cible.

**Fini quand** — Un déplacement sur la grille montre le personnage
marcher dans la bonne direction, chez tous les joueurs et sur la TV.

**Origine** — Romain, session de design du 3 octobre 2026

**État** — Chaque pièce des deux packs se dessine maintenant de profil,
de face et de dos (la gauche reste le miroir du profil) : les corps,
cheveux, barbes et coiffes de face et de dos sont dessinés à la main,
le reste (tenues, armures, armes, accessoires) a été tourné une fois
depuis le profil par une règle par profondeur
(`docs/design/sprite-turn.py`), et les packs sont redevenus la source
qu'on édite à la main. Un pack qui oublie une direction est refusé ; une
liste vide dit « on ne la voit pas de ce côté » (une barbe de dos). Le
serveur dessine, pour chaque allure et chaque direction, une planche de
quatre images (`GET /api/sprites/sheet.png`) : repos, respiration et
deux pas — jambes écartées de profil, un pied levé de face et de dos
(planche « Sprite » du canevas : `repos`, `marche`). Sur la carte, les
pions sont ces personnages (et plus des disques) : ils respirent au
repos, marchent case par case le chemin de leur dernier déplacement en
se tournant à chaque pas, se fendent vers la cible qu'ils frappent et
clignotent quand ils sont touchés ; immobiles si l'appareil demande
moins d'animations. Le serveur tient l'orientation et le dernier
déplacement de chaque pion (déplacement du joueur, pion posé par le MJ,
pas et coups du combat), si bien que chaque écran rejoue le même
déplacement ; ce qu'un écran voit en s'ouvrant n'est jamais rejoué, et
un déplacement passé par le brouillard n'est montré aux joueurs que sur
les cases qu'ils voient. Les PNJ prennent l'allure de leur monde (un
« marin 2 » est un marin), et un PNJ sans allure (campagne générée) en
reçoit une tirée de son identifiant, la même partout. Le créateur montre
l'aperçu en mouvement, tournable dans les quatre directions, avec un
bouton « Marcher » ; `/reference` montre chaque personnage des deux
mondes marchant dans les quatre directions. Testé sur les deux mondes
(le quai de Port-Louis, la coursive du Cure-Dent) jusqu'au combat, et
essayé dans un navigateur au format téléphone sur le quai. Aucune
migration : l'orientation et le dernier déplacement vivent dans les
pions déjà stockés en JSON.
**Hypothèse** : l'attaque et le touché sont des mouvements du sprite
entier (fente, clignotement, recul), comme sur la planche « Sprite », et
pas des images dessinées par pièce. **Limite** : la TV
(`tv/show-evening`, phase 5) n'existe pas encore ; elle reprendra le
même dessin de carte et les mêmes planches, et c'est là qu'il faudra
voir la marche. **Reste** : un déplacement joué pour de vrai, vu en même
temps sur plusieurs téléphones ; la TV ; Romain regarde les vues de face
et de dos (`shared/tests/golden/<monde>/marche.png`, ou `/reference`) et
dit ce qui cloche.

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

### `maps/model-grid-maps` · doing — reste le branchement du déplacement sur le système de règles

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

**État** — Modèle et règles livrés dans `shared/src/maps/`, format
décrit dans `docs/map-format.md` (YAML pour écrire, même forme en JSON
pour la base). Une carte = échelle, légende de glyphes + rangées
(terrain, mur, eau peu profonde ou profonde, terrain difficile, vide,
hauteur), portes (ouverte, fermée, verrouillée ; une porte secrète est
une porte sur un calque MJ, sa case reste un mur pour les joueurs),
décors avec emprise et règles (couvert ½, ¾ ou total, bloque le passage,
difficile, escaladable), objets à trouver (jet et notes réservés au MJ),
lumières, ambiance (heure, météo, lumière de base, portée de vue, humeur,
vent), sorties vers d'autres cartes, étiquettes, positions de départ, et
calques visibles de tous, du MJ seul, ou des joueurs seuls (illusion,
ignorée par les règles). Le moteur calcule cases atteignables, validation
d'un chemin envoyé par le client, portée, ligne de vue avec niveau de
couvert, cases vues depuis un point (pour le brouillard) et éclairage ;
`Map::project` est le point unique qui retire aux joueurs ce qu'ils ne
doivent pas voir. Hexagones : coordonnées, distance, voisins et
déplacement d'un pas par hexagone pour le monde. Les deux cartes témoins
sont écrites (`content/maps/corsaires/quai-port-louis.yaml`,
`content/maps/brasier/cure-dent-coursive.yaml`, six contre six) et les
quatre cartes de la démo V1 se chargent (`content/maps/v1-demo/`,
chargeur `load_v1_story_maps`). 34 tests dans `shared/`. Les cartes
propres à une campagne sont stockées en base (`campaign_maps`, migration
019, avec `maps/edit-map-gm`). Reste : le branchement des paramètres de déplacement (`MovementRules`) sur le
système de règles (`engine/model-rule-system`). Le rendu, le brouillard,
les véhicules (orientation, arcs) et le voyage en hexagones lisent ce
modèle sans le changer.

**Origine** — Session de design du 3 octobre 2026 · V1 (format YAML des cartes)

### `maps/render-three-quarter-tiles` · doing — reste la soirée du jalon jouée pour de vrai

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

### `maps/blend-outdoor-terrain` · doing — reste la soirée du jalon jouée pour de vrai

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

### `maps/build-tileset-packs` · doing — reste la soirée du jalon jouée pour de vrai

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

### `maps/package-theme-packs` · doing — reste la soirée du jalon jouée pour de vrai

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

### `maps/edit-map-gm` · doing — reste une carte de rencontre complète faite par le MJ sur l'app, et le pinceau de brouillard

**Pourquoi** — Le MJ garde le dernier mot sur chaque carte.

**Périmètre** — Éditeur dans l'écran MJ : peindre terrains et murs,
poser décors, portes, objets cachés et lumières, régler l'ambiance,
pinceau de brouillard, révéler un calque ou un objet en direct.

**Fini quand** — Le MJ crée une carte de rencontre complète en moins
de dix minutes et révèle un passage secret pendant la session ; les sept
moments de la planche « Cartes · l'éditeur » sont faisables dans l'app.

**État** — Page « Cartes » de la campagne (`/campagnes/:id/cartes`) :
liste des cartes de la campagne, nouvelle carte vide (16 × 16, murée)
ou copie d'une carte du monde. Éditeur (`/campagnes/:id/cartes/:carte`) :
pinceaux sol, mur, eau, vide (glisser), portes (fermée → ouverte →
verrouillée → retirée ; la case devient un mur), décors du jeu de tuiles,
objets cachés, lumières, départs héros et adversaires, gomme ; tout peut
aller sur le calque secret ; heure, météo, lumière de base et notes du
MJ. Enregistrer repasse la carte en brouillon ; seule une carte validée
est proposée au plateau (avant les cartes du monde) et peut être montrée.
Le serveur valide chaque enregistrement et répond en français. Une carte
affichée à la table ne peut pas être supprimée. Révéler un calque ou un
objet en direct existait déjà (`maps/reveal-fog-and-hidden`). Écart : pas
encore de pinceau de brouillard. Pas encore essayé sur une vraie
préparation.

**Origine** — Planches « Cartes · trois échelles » et « Cartes · l'éditeur »

### `maps/generate-map-llm` · doing — reste une vraie carte générée avec la clé OpenRouter puis validée

**Pourquoi** — Préparer une carte doit être aussi rapide que décrire
la scène.

**Périmètre** — À partir d'une scène du graphe d'histoire, le LLM
propose une grille (murs, portes, terrains, décors, objets cachés)
dans le jeu de tuiles du décor ; le MJ la relit dans l'éditeur et la
valide. Rien n'arrive aux joueurs sans validation ; chaque appel est
compté.

**Fini quand** — Une scène de la démo produit une carte jouable,
validée par le MJ, en une génération et quelques retouches.

**État** — Sur la page « Cartes », le MJ choisit une scène (les
combats d'abord) et le co-MJ propose une carte (gabarit
`map-generation.v1`, appel compté `map.generate`) : il reçoit la scène,
ses adversaires, les matériaux et décors du jeu de tuiles, les
caractéristiques du système de règles et la taille du groupe. Le serveur
impose version, identifiant, décor, échelle et calques, retire image de
fond et sorties, écarte un adversaire ou une caractéristique inventés
(décomptés dans `dropped`) ; si la carte reste invalide, une seconde
chance avec les raisons en français, puis une erreur. La proposition
arrive en brouillon dans l'éditeur et n'atteint la table qu'une fois
validée. Écart : la carte n'est pas encore rattachée automatiquement au
nœud (le MJ la choisit au plateau).

**Origine** — Règles de design 1 et 2 (`CLAUDE.md`)

### `maps/import-image-map` · doing — reste un vrai `.dd2vtt` et une vraie image importés et joués

**Pourquoi** — Beaucoup de MJ ont déjà des cartes (achetées, faites
dans Dungeondraft ou Dungeon Alchemist, générées).

**Périmètre** — Importer ou générer une image ; aligner la grille
(taille de case, décalage, case témoin) ; tracer les murs, ou laisser
l'IA les proposer puis les valider ; importer le format Universal VTT
(`.dd2vtt`) avec ses murs et lumières ; l'image n'est qu'un décor,
tout ce qui est caché ou change d'état est un objet posé par-dessus.

**Fini quand** — Une carte `.dd2vtt` et une image brute deviennent
jouables, avec ligne de vue correcte, en moins de cinq minutes.

**État** — Import sur la page « Cartes ». Un fichier Universal VTT
(`.dd2vtt`, `.uvtt`, `.df2vtt`) devient une carte : murs, portes et
lumières lus du fichier, image gardée en fond. Une image (PNG, JPEG,
WEBP, 25 Mo au plus) se cale sur la grille (taille de case, décalage,
aperçu de la grille) et devient un sol ouvert à tracer dans l'éditeur,
où l'image reste sous la grille. Le fond est servi au MJ et, une fois la
carte montrée, aux joueurs. Écarts : les murs Universal VTT sont des
segments, Promptus les pose sur des cases (une case traversée devient un
mur, une porte rend sa case mur) ; l'IA ne propose pas encore les murs
d'une image brute ; pas de case témoin.

**Origine** — Recherche sur Foundry VTT et Owlbear Rodeo, session du 3 octobre 2026

### `maps/reveal-fog-and-hidden` · doing — reste la soirée du jalon jouée pour de vrai

**Pourquoi** — Explorer, c'est découvrir : ce qui n'est pas vu ne doit
pas fuiter.

**Périmètre** — Brouillard (motif de petits carrés) révélé par la
ligne de vue ou par le MJ ; objets cachés visibles du seul MJ ; un
joueur invisible absent de la TV mais visible en fantôme sur son
téléphone. Tout est filtré côté serveur avant l'envoi.

**Fini quand** — Un test prouve qu'aucune donnée d'une case non
révélée ou d'un objet caché n'atteint un client joueur.

**Origine** — `MEMORY.md` §3 (projection joueur)

### `maps/travel-hex-world` · doing — reste un vrai voyage joué à une table, sur un téléphone, avec la TV

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

*Hypothèse retenue* — Valombre et Morneval sont l'exemple de la planche,
pas un monde à construire : le même voyage se joue sur les deux mondes
témoins. Les Corsaires vont de Port-Louis au Palais (Belle-Île) sur la
carte des côtes de Bretagne sud, La Mâchoire en pion ; le Brasier va du
Toboggan au Reliquaire, station sereth, sur la carte du système, le
Cure-Dent en pion.

**État** — Moteur dans `shared/src/travel/` : routes vers un lieu (la
moins chère et une seconde qui s'en écarte), portions de journée au pas
de chaque terrain (le reste se reporte, un hexagone cher prend plusieurs
portions), nuit après la dernière portion, vivres mangés à l'aube, tables
d'événements par terrain tirées sans répétition. La vitesse, les
portions, les vivres, les tours de garde et les tables vivent dans un
guide de voyage à côté de la carte (`content/travel/`, format dans
`docs/map-format.md`) ; une carte du monde sans guide se parcourt selon
ses cases. Deux cartes du monde et deux cartes de lieu écrites
(`cotes-bretagne-sud`, `le-palais`, `systeme-brasier`,
`reliquaire-sereth`, INVENTÉ — à valider par Romain). Serveur : table
`travels` (migration 044) par campagne et carte du monde : le pion du
groupe, les hexagones vus, le jour, les vivres et le voyage en cours,
recopiés sur le plateau dans la même transaction ; revenir sur la carte
du monde la retrouve telle qu'on l'a laissée. Personne ne traîne le pion
(`TRAVEL_MAP`) ; le brouillard s'y lève autour des hexagones traversés.
Les sept moments : le MJ touche un lieu, le serveur propose deux routes
que le MJ renomme et décrit ; chaque joueur vote sur son téléphone, le
MJ voit qui veut quoi et choisit (le vote conseille, il ne décide pas) ;
« Portion suivante » avance le pion, « Perdre une portion » fait passer
le temps sans bouger ; après chaque portion, trois événements de la
table du terrain arrivent au seul MJ, qui en garde un (texte retouché)
ou aucun ; jet de groupe lancé par le MJ, chaque joueur lance sur son
téléphone, le serveur applique le seuil du système (au moins la moitié
pour les deux mondes) et l'XP ; la nuit, le MJ fixe les tours de garde
et parle au seul veilleur, qui peut réveiller les autres ; à l'arrivée,
« Entrer » ouvre la carte du lieu (la sortie posée sur son hexagone) et
sa scène (sur le Brasier, `sc_carapace_sereth`). Les routes proposées
sont montrées entières aux joueurs, brouillard compris ; un lieu secret
visé par le MJ reste sans nom pour eux. Tests : 11 dans `shared`, 3 sur
l'API pour les deux mondes (y compris l'absence de fuite des notes MJ,
des événements non gardés et du mot au veilleur), 6 à l'écran. Écarts :
« le co-MJ propose » tire dans les tables écrites, sans appel au modèle
(pas d'événement inventé à partir des fronts) ; pas d'écran TV
(`tv/show-evening`, phase 5) ; le pion est un losange, pas encore le
sprite du groupe ; les tables d'événements d'une carte du monde faite
par le MJ ne s'éditent pas encore dans l'app ; aucune horloge de front
n'avance seule avec les jours. Pas encore joué pour de vrai.

**Origine** — Planches « Cartes · trois échelles » et « Voyager »

### `maps/support-hex-combat` · todo · à spécifier

**Pourquoi** — Certains MJ préfèrent les hexagones aussi en combat.

**Périmètre** — Seconde géométrie pour le moteur de règles
(déplacement, portée, ligne de vue) et le rendu ; à décider après les
premières sessions réelles.

**Origine** — Session de design du 3 octobre 2026 (reporté volontairement)
