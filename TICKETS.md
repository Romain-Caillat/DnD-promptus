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
| `hd2d` | Le plateau en HD-2D façon Octopath : scène Three.js, éléments PixelLab, combat mis en scène |

## Ordre de construction

**Priorité de Romain (11 octobre 2026).** Le travail porte d'abord sur
la nouvelle création de campagne, branchée sur PixelLab et le rendu
HD-2D : les trois tickets vont ensemble et passent par Claude :
`campaign/prepare-with-claude` (préparer en conversation),
`campaign/import-pixellab-assets` (Claude génère chez PixelLab et dépose
dans Promptus), `campaign/watch-prep-in-hd2d` (l'Atlas devient le
visualiseur HD-2D), avec l'épic `hd2d` qui les porte. L'ordre :
le premier, puis `hd2d/render-board-in-three` et le troisième, puis le
deuxième une fois l'abonnement PixelLab pris. Le reste de l'ordre
ci-dessous attend. Les trois sont construits (11 octobre 2026) ; restent
leurs essais réels, et la TV qui attend `hd2d/render-board-in-three`.

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

**Phase 0 · Le socle** — `platform/deploy-self-hosted` ·
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

### Jalon 4 · Préparer sur la carte, avec Claude en direct

La préparation quitte les listes pour la carte : un **Atlas** (région en
hexagones → lieu → rencontre) où les scènes sont épinglées, le fil de
l'histoire se lit comme un chemin, PNJ, indices et combats sont posés à
leur place, avec le rendu du jeu. Claude, par le serveur MCP, y travaille
en direct : le MJ le voit poser ses propositions en pointillés et les
garde ou les jette. Planche : « Atlas · préparer avec Claude en direct ».

**Ce qui ne change pas** — Le graphe d'histoire reste la source : la
carte en est une vue, le fil se calcule depuis les sorties des scènes,
jamais dessiné (`MEMORY.md`, « l'histoire est un graphe, pas un
script »). Le validateur, la jauge, la règle des trois indices et les
outils MCP existants restent valables.

**Fini quand** — Romain prépare un acte d'un des deux mondes dans
l'Atlas, en demandant à Claude de peupler deux lieux et un combat ; il
voit chaque proposition apparaître sans recharger, en garde une partie,
vérifie la vue joueurs, puis joue l'acte à une vraie soirée.

**Ordre retenu** — Après la soirée du jalon 1, jouée avec les écrans
actuels : ce virage ne la remplace pas.

**Phase 7 · Les scènes sur la carte** — `campaign/pin-scenes-on-maps` ·
`maps/browse-atlas` · `campaign/draw-story-thread` ·
`campaign/place-cast-and-clues` · `maps/preview-as-player` ·
`gm/prep-on-atlas`

**Phase 8 · Claude en direct** — `maps/edit-maps-by-ops` ·
`platform/stream-prep-edits` · `platform/propose-edits-by-claude` ·
`platform/edit-maps-by-claude`

**Ordre de construction** — Il suit les dépendances, pas la numérotation
des phases : la phase 8 pose d'abord le socle. `platform/stream-prep-edits`
(révision et journal communs, diffusion au MJ seul) →
`maps/edit-maps-by-ops` (les cartes entrent dans ce même journal) →
`maps/browse-atlas` et la coquille de l'écran Atlas →
`campaign/pin-scenes-on-maps` → `campaign/place-cast-and-clues` →
`campaign/draw-story-thread` → `platform/propose-edits-by-claude` →
`platform/edit-maps-by-claude` → `maps/preview-as-player` puis la bascule
`gm/prep-on-atlas` (les anciens écrans Préparer, Médias, Cartes et Vue
joueurs disparaissent).

**Hypothèses retenues** (validées par Romain) — L'épingle est portée par
le **lieu** de l'histoire (`Location`, qui a déjà un parent) : un lieu
gagne une ancre facultative sur une carte (carte + case ou zone), et la
chaîne des parents donne région › lieu › rencontre ; plusieurs scènes
partagent un lieu. L'étiquette de carte qui nomme une scène ne devient
pas une seconde source. Les positions de PNJ, indices et rencontres sont
de la préparation, rangées dans l'histoire (pas dans l'état de carte de
la soirée), recopiées en pions quand la soirée ouvre la carte. **Un seul**
système de révisions par campagne (numéro + journal : auteur, changements,
inverse) pour l'histoire et les cartes. Le temps réel ne porte que des
numéros ; la révision de préparation et l'activité de Claude ne partent
que vers les sockets du MJ. Tout nouveau champ du format est facultatif :
un ancien fichier s'importe toujours.

*Tickets existants touchés* — `campaign/edit-scenes-in-one-place` (on y
entre aussi par une épingle) ; `campaign/check-act-readiness` (alerte
« scène sans lieu ») ; `maps/generate-map-llm` (part d'une épingle et
rattache la carte à la scène) ; `maps/edit-map-gm` (passe aux
opérations, outil épingle) ; `campaign/review-story-graph` (le graphe
devient une vue secondaire) ; `platform/preview-campaign-in-claude`
(gelé après son premier panneau : la vue en direct devient l'app).

Plus tard, sans jalon : `maps/support-hex-combat`.

**Sprites générés (essai d'octobre, hors jalon)** —
`media/generate-sprites-on-mannequin` · `media/draw-key-poses` ·
`media/write-pixel-art-skill` · `characters/show-generated-sprites-on-plan`
· `characters/choose-sprite-source` (tranché le 11 octobre : HD-2D
avec des éléments PixelLab).

**HD-2D (décidé le 11 octobre, pas encore placé dans un jalon)** —
`media/choose-pixellab-plan` · `hd2d/build-pixellab-asset-pack` ·
`hd2d/render-board-in-three` · `hd2d/show-pixellab-characters` ·
`hd2d/stage-combat-actions` · `hd2d/retire-flat-renderers`. Priorité
avec la nouvelle création de campagne (voir « Priorité de Romain » en
tête de l'ordre de construction).

---

## Épic `platform`

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

### `platform/preview-campaign-in-claude` · gelé — le premier panneau (récap des scènes et inventaire des assets) n’a jamais été construit ; le reste est abandonné

**Pourquoi** — Quand Romain prépare sa campagne avec Claude Code
(`platform/connect-claude-mcp`), il ne voit le résultat qu'en ouvrant
Promptus ou en lisant le fichier de campagne. Il veut voir, à côté de
la conversation et pendant que Claude écrit, ce qui a été produit :
les scènes créées, leur résumé, leurs assets, et ce qui manque encore
pour la soirée. Un second panneau fera de même pour les règles.

**Périmètre** — Des mods Claude Code (panneaux dessinés dans la session,
à côté de la conversation) qui montrent la campagne en lecture seule.
Ils ne modifient rien : écrire reste le travail de Claude par le
serveur MCP, et valider reste le geste du MJ dans Promptus. Pistes
retenues, à trier en critères d'acceptation :

*Voir ce qui existe*
1. **Récap des scènes** — une carte par scène, rangée par acte : titre,
   résumé, lieu, accroche, sorties, ambiance. La scène que Claude vient
   de créer ou de modifier est mise en avant ; les passages « INVENTÉ —
   à valider » sont signalés.
2. **Graphe de l'histoire** — les scènes et leurs sorties dessinées :
   scène d'ouverture, impasses, scènes jamais atteintes, scènes
   facultatives.
3. **Distribution** — quel PNJ, quelle faction, quel lieu apparaît dans
   quelles scènes ; le PNJ jamais utilisé, le méchant absent d'un acte.

*Voir ce qui manque*
4. **Inventaire des assets** — par scène : carte présente ou non,
   musique choisie ou encore « à chercher », visuels décrits ou déjà
   générés. Le résultat est une liste de ce qui reste à produire avant
   la soirée.
5. **Jauge de préparation par acte** — prêt, presque, ébauche, avec la
   raison, en relisant la jauge que Promptus calcule déjà
   (`campaign/check-act-readiness`) plutôt qu'en la recalculant.
6. **File de validation** — tout ce que Claude a inventé et que le MJ
   doit relire, élément par élément.

*Vérifier que l'histoire tient*
7. **Révélations et indices** — chaque révélation, ses indices et la
   scène où on les trouve ; une révélation qui a moins de trois indices
   est signalée.
8. **Menaces et horloges** — chaque menace, ses étapes, les scènes qui
   la font avancer ; on voit si la catastrophe peut réellement arriver.
9. **Vue joueur** — une scène vue par le MJ et par les joueurs côte à
   côte (projection joueur) : aucun secret ne doit fuiter.

*Les règles (second panneau)*
10. **Règles** — classes, actions, états, alertes du lint
    (`engine/lint-rule-system`) et résultat des combats simulés
    (`engine/simulate-fights`).

*Proposition* — commencer par un seul panneau qui réunit le récap des
scènes et l'inventaire des assets (pistes 1 et 4) : il répond à
« qu'est-ce que j'ai ? » et « qu'est-ce qui me manque pour la
soirée ? ». Révélations et menaces (7, 8) en deuxième onglet, les
règles (10) dans un panneau à part.

*Hypothèse* — La source est la campagne dans Promptus, lue avec le
jeton du MJ par les mêmes routes que le serveur MCP (export, jauge,
alertes), pour voir ce que Claude a réellement écrit dans la
plateforme. Le fichier YAML local (`content/campaigns/`) sert de repli
quand Promptus n'est pas lancé. Le panneau se rafraîchit quand Claude
appelle un outil d'écriture du serveur MCP.

**Étapes** — Choisir les pistes du premier panneau et écrire leurs
critères → panneau et commande dans Claude Code → lecture de la
campagne (Promptus, puis repli fichier) → rafraîchissement sur les
écritures de Claude → essai réel en préparant un acte d'un des deux
mondes → onglets suivants, puis le panneau des règles.

**Fini quand** — Romain demande à Claude Code d'ajouter une scène à
une campagne des deux mondes, et le panneau la montre aussitôt avec son
résumé, ses assets et ce qui lui manque, sans ouvrir Promptus ni le
fichier.

**Risques** — Les mods reposent sur une interface de Claude Code en
accès anticipé, qui peut changer d'une version à l'autre. Le jeton du
MJ ne doit jamais s'afficher dans un panneau ni dans un journal. La
façon dont un mod appelle l'API de Promptus n'a pas encore été vérifiée.
Un panneau qui recalcule ce que le serveur calcule déjà (jauge, alertes,
projection) finirait par diverger : il relit le serveur.

**Origine** — Demande de Romain, 10 octobre 2026.

*État* — Premier panneau livré (pistes 1 et 4) : `claude-mods/promptus-scenes`,
commande `/promptus`. Il lit les fichiers de campagne du dépôt (pas
encore Promptus) et se met à jour quand Claude ou le MJ modifie le
fichier. Pas encore regardé dans une vraie séance de préparation.

*Décision* (`gm/prep-on-atlas`, 11 octobre 2026) — Gelé après son
premier panneau, comme le jalon 4 l’annonçait : la vue en direct est
devenue l’app. L’Atlas montre déjà, pendant que Claude écrit, ce qu’il
propose (en pointillés sur la carte, bandeau « Claude écrit · sur … »),
le fil des scènes, la jauge, les alertes et la vue joueurs. Aucun
panneau n’existait encore dans Claude Code : si le besoin revient, seul
le premier panneau (récap des scènes et inventaire des assets, pistes 1
et 4) reste à faire, sans recalculer ce que Promptus calcule.

---

### `platform/stream-prep-edits` · doing — reste l'essai réel : Claude écrit depuis Claude Code, l'Atlas ouvert le montre sans recharger (bandeau sur la carte, « Modifié » dans le tiroir)

**Pourquoi** — Quand Claude modifie la campagne par le serveur MCP,
l'Atlas ouvert du MJ doit le montrer aussitôt, sans recharger.

**Périmètre** — Chaque écriture de préparation (MJ, jeton, co-MJ)
incrémente un numéro de révision de la campagne et notifie le sujet
concerné sur le canal temps réel existant ; le canal ne transporte
toujours que des numéros, l'écran relit par HTTP. Un journal des
révisions (auteur, modifications, inverse) permet de rattraper ce qui a
changé depuis N et d'annuler une révision tant que rien ne l'a
recouverte. Un signal d'activité « Claude écrit · sur Valombre » part
vers le MJ **seulement** : aujourd'hui la présence part vers toutes les
sockets de la campagne, joueurs compris.

**Fini quand** — Claude ajoute un indice depuis Claude Code et l'Atlas
ouvert le met en avant sans rechargement ; un téléphone joueur connecté
à la même campagne ne reçoit ni l'activité ni la révision.

**Étapes** — Numéro de révision de la campagne et journal (une ligne par
écriture : auteur MJ, jeton ou co-MJ, ce qui a changé par identifiant,
l'inverse), écrits au point unique où l'histoire est enregistrée, donc
par toutes les écritures (modifications, import, atelier, génération,
réglages, montée de version des règles) → route « ce qui a changé depuis
N » et annulation d'une révision tant qu'aucune autre n'a touché les
mêmes éléments → sujet temps réel réservé au MJ, filtré par spectateur
(y compris l'état envoyé à la connexion) et signal d'activité quand un
jeton écrit → écrans de préparation du MJ qui relisent à chaque révision
et mettent en avant ce qui a changé.

**Vérification** — Tests serveur : chaque écriture ajoute une révision
avec le bon auteur ; le rattrapage rend ce qui a changé depuis N ;
l'annulation rétablit l'état et est refusée quand une révision plus
récente a touché les mêmes éléments ; une socket joueur et une socket
TV ne reçoivent ni la révision ni l'activité, même à la connexion.
Tests front : l'écran relit et met en avant. Essai réel : Claude Code
écrit, l'écran ouvert le montre sans recharger.

**Risques** — Une fuite de l'activité de Claude vers les joueurs
révèle la préparation : un test sur la socket joueur le vérifie. Un
journal qui garde l'état d'avant grossit avec la campagne : à surveiller,
élaguer plus tard si besoin.

**Origine** — Demande de Romain, 10 octobre 2026 ; planche « Atlas · préparer avec Claude en direct » (page « MJ · Créer, préparer »).

*État* — Chaque écriture de la préparation, quel que soit son auteur,
porte désormais un numéro de révision et laisse une ligne dans le journal
de la campagne : qui a écrit (le MJ dans Promptus, Claude par son jeton,
le co-MJ quand le MJ accepte une proposition de l'atelier ou une
génération, le serveur quand il adopte une version des règles à
l'ouverture d'une soirée), quels éléments ont bougé, et chacun tel qu'il
était. Une écriture qui ne change rien ne prend pas de numéro. Le MJ
peut demander « ce qui a changé depuis N » et annuler une révision tant
qu'aucune révision plus récente n'a touché les mêmes éléments ;
l'annulation remet seulement ce que cette révision avait touché, garde
le reste, et devient elle-même une révision (on peut donc la défaire à
son tour) ; ce que le serveur a fait seul ne s'annule pas, et un jeton ne
peut ni lire le journal ni annuler. L'écran de préparation ouvert relit
la campagne à chaque révision, met « Modifié » sur les scènes et fiches
que Claude ou le co-MJ viennent de toucher, et affiche « Claude écrit ·
sur … » avec un bouton Annuler tant que c'est possible. Les téléphones
des joueurs et la TV ne reçoivent ni la révision ni l'activité de Claude,
pas même à la connexion (testé sur une socket joueur et une socket TV) ;
la présence (qui est connecté) part toujours vers tous, comme avant.
Depuis la revue de l'Atlas, le bureau du MJ (cartes de la campagne,
brouillons du co-MJ, images à valider, brouillon de règles, tour adverse
proposé) est lui aussi réservé au MJ : quand Claude touche une carte,
qu'il la propose ou l'écrive directement avec la main, rien ne bouge sur
un téléphone ni sur la TV, qui ne se recharge plus pendant la
préparation (testé sur une socket joueur et une socket TV).
**Hypothèses retenues** : le journal garde l'état d'avant de chaque
élément touché (pas de diff fin par champ) ; seul l'écran Préparer met en
avant les changements pour l'instant, l'Atlas le reprendra ; l'activité
ne porte qu'un numéro, le nom du lieu est lu dans le journal. **Reste** :
l'essai réel depuis Claude Code. Les opérations de carte entrent
désormais dans ce même journal et ce même numéro (`maps/edit-maps-by-ops`,
cible `map:<id>`) ; l'écran Préparer affiche « Claude écrit · sur <nom de
la carte> » pour un lot de carte, faute d'Atlas. Depuis
`platform/propose-edits-by-claude`, l'Atlas montre le même bandeau ; une
écriture de Claude n'y arrive directement que si le MJ lui a donné la
main, sinon elle s'affiche comme une proposition en pointillés.

---

*Depuis `gm/prep-on-atlas`* — L’écran Préparer n’existe plus : l’Atlas
porte le bandeau « Claude écrit · sur … » et son Annuler, et son tiroir
met « Modifié » sur les scènes et fiches touchées. L’essai réel se fait
donc dans l’Atlas, carte ou tiroir ouvert.

### `platform/propose-edits-by-claude` · doing — reste l'essai réel avec Claude Code : Romain demande de peupler un lieu, voit les propositions en pointillés sans recharger, en jette une avec un mot, Claude en propose une autre, il garde le reste, rien n'a été visible des joueurs

**Pourquoi** — Aujourd'hui une écriture par jeton vaut une écriture du
MJ et s'applique tout de suite. Dans l'Atlas, Claude propose et le MJ
garde : c'est la règle 1, rendue mécanique.

**Périmètre** — Les modifications de Claude vont dans un lot (titre,
foyer sur la carte), appliqué virtuellement pour l'affichage : l'Atlas
les dessine en pointillés marqués « ▶ Claude ». Le MJ garde, jette avec
un mot, ou garde tout ; accepter rejoue le lot sous le verrou de la
campagne et refuse ce qui est devenu impossible. Seule la session du MJ
peut accepter, jamais un jeton. Un mot de rejet revient à Claude, qui
peut proposer autre chose. Les outils MCP d'écriture passent par ce lot.

*Décision ouverte* — un mode « Claude a la main » (écriture directe,
annulable par lot) hors soirée ouverte ? Recommandation : oui, réglé par
le MJ, refusé pendant une soirée et sur un contenu déjà révélé. Tranche
la décision ouverte de `platform/connect-claude-mcp`.

**Fini quand** — Romain demande à Claude de peupler le cimetière ;
quatre éléments apparaissent en pointillés, il en jette un avec « trop
évident », Claude en propose un autre, Romain garde le reste ; rien n'a
été visible des joueurs avant.

**Étapes** — Une écriture par jeton ne s'applique plus : elle crée un lot
de propositions (on réutilise les propositions de l'atelier, accepter ou
rejeter) avec son foyer sur la carte → l'Atlas dessine le lot appliqué
virtuellement, en pointillés → garder, jeter avec un mot, tout garder :
seule la session du MJ accepte, sous le verrou, et ce qui ne s'applique
plus est refusé un par un → le mot de rejet revient à Claude par un outil
MCP de lecture → mode « Claude a la main », réglé par le MJ, refusé
pendant une soirée ouverte et sur un contenu déjà révélé, annulable par
lot grâce au journal des révisions.

**Vérification** — Tests serveur : un jeton ne change jamais l'histoire
hors du mode « a la main » ; un jeton ne peut pas accepter ; accepter
rejoue sous le verrou et refuse ce qui est devenu impossible ; le mode
est refusé soirée ouverte. Test MCP : le mot de rejet est lu. Essai réel
avec Claude Code.

**Risques** — Un lot qui vieillit pendant que le MJ édite : accepter
revalide tout contre l'état présent. Deux circuits de proposition (atelier
du co-MJ, lot de Claude) qui divergent : un seul modèle de lot.

**Origine** — Demande de Romain, 10 octobre 2026 ; planche « Atlas · préparer avec Claude en direct » (page « MJ · Créer, préparer »).

*État* — Quand Claude écrit avec le jeton du MJ, que ce soit une
modification de l'histoire (une scène, un PNJ, une épingle, un indice
posé) ou des gestes sur une carte, rien ne change dans la campagne : le
serveur vérifie la demande comme si le MJ l'avait faite (une erreur
revient tout de suite à Claude), puis la range dans un lot de
propositions, avec le titre que Claude lui donne et l'endroit de la
carte dont il parle. Claude peut ajouter à un lot encore en attente.
Dans l'Atlas, le panneau commence par « Propositions de Claude » : chaque
lot liste ses éléments ; le MJ en garde un, en jette un en laissant un
mot (« trop évident »), ou garde ou jette tout ce qui reste ; « Voir sur
la carte » l'emmène là où le lot travaille. Sur la carte, ce que le lot
ajouterait (épingles, PNJ, indices, combats, flèches du fil, cases
peintes) est dessiné en pointillés et marqué ▶ ; le toucher ouvre son lot.
Garder rejoue l'élément sur la campagne telle qu'elle est à cet instant :
un élément devenu impossible (le PNJ a été supprimé entre-temps, une case
que le MJ a repeinte) est refusé et le dit, le reste est gardé. Seul le
MJ, dans Promptus, peut garder ou jeter : un jeton ne le peut jamais.
Claude relit le sort de ses lots et les mots du MJ avec un nouvel outil
(« Lire la réponse du MJ »), et l'outil « Modifier l'histoire » lui dit
désormais si c'est proposé ou appliqué, et pourquoi. L'écran Préparer
signale les lots en attente et mène à l'Atlas ; l'atelier du co-MJ ne
montre que les propositions du co-MJ. Rien d'un lot n'arrive aux joueurs
ni aux écrans partagés, pas même le fait qu'il existe (testé).
Le réglage « Claude a la main », dans le même panneau, laisse les
écritures de Claude s'appliquer directement, chacune annulable depuis le
bandeau « Claude a écrit · sur … » (une demande de Claude, une ligne du
journal). Il ne peut pas être activé pendant une soirée ouverte ; et même
activé, une écriture qui arrive pendant une soirée, ou qui touche ce que
les joueurs connaissent déjà (une scène où la table est entrée, un indice
trouvé, un PNJ rencontré, la carte montrée), reste proposée. Remplacer
toute la campagne par un fichier demande la main. Cela tranche la
décision ouverte de `platform/connect-claude-mcp`.
*Interprétations à confirmer* : un élément est une modification ou un
lot de gestes sur une carte (pas un regroupement par PNJ ou lieu) ;
garder un élément est attribué au co-MJ dans le journal (« une
proposition acceptée », comme l'atelier), pour que le geste du MJ ne
s'affiche pas comme « Claude écrit » ; « annulable par lot » en mode
main veut dire une demande de Claude = une ligne du journal, annulée
comme les autres ; une suppression proposée n'est pas dessinée sur la
carte, seulement listée. **Reste** l'essai réel depuis Claude Code. Le
« Fini quand » parle du cimetière, qui n'existe dans aucun des deux
mondes : proposition, faire l'essai sur Port-Louis (peupler le quartier
des rues) à trancher par Romain.

---

### `platform/edit-maps-by-claude` · doing — reste l'essai réel depuis Claude Code : « ajoute un PNJ dans un lieu et deux indices sur un chemin » donne trois propositions bien placées, sans mur traversé ni PNJ dans l'eau

**Pourquoi** — Claude ne voit pas les pixels : pour travailler sur
l'Atlas il lui faut la carte en texte et des gestes qui ont du sens.

**Périmètre** — Outils MCP de lecture : l'atlas (cartes et lieux), une
carte ou une zone en ASCII avec légende et éléments posés, les questions
au moteur (chemin et coût, ligne de vue, ce que voient les joueurs). En
écriture, des opérations sémantiques résolues et validées par le serveur
— épingler une scène, poser un PNJ « près du comptoir », un indice, une
rencontre — plutôt que des coordonnées de cases. Les routes s'ajoutent à
`TOKEN_ROUTES` (décision explicite, `MEMORY.md`) ; tout passe par le lot
de `platform/propose-edits-by-claude`.

**Fini quand** — Depuis Claude Code, « ajoute Oswin au cimetière et deux
indices sur le chemin de la colline » produit trois propositions bien
placées sur la carte de Valombre, sans mur traversé ni PNJ dans l'eau.

**Étapes** — Outils de lecture (atlas, carte ou zone en ASCII avec
légende, questions au moteur) → opérations sémantiques résolues par le
serveur (épingler, poser près de, poser sur le chemin) → ajout des routes
de cartes à la liste des routes d'un jeton, décision consignée dans
`MEMORY.md` (l'invariant « un jeton n'atteint que la préparation » est
réécrit) → tout passe par le lot de `platform/propose-edits-by-claude`.

**Vérification** — Le balayage des routes MJ reste vert avec la liste
élargie ; tests serveur : une pose « près du comptoir » tombe sur une
case libre et atteignable, jamais dans un mur ou dans l'eau ; tests MCP
des outils. Essai réel depuis Claude Code.

**Risques** — Le raisonnement spatial des modèles est faible sur les
grilles : le serveur résout les positions, Claude choisit le sens. Un
rendu en image pour Claude est reporté, à décider après une vraie
séance. Élargir les routes d'un jeton élargit ce qu'une fuite permet :
écritures toujours en propositions, jamais pendant une soirée.

**Origine** — Demande de Romain, 10 octobre 2026 ; planche « Atlas · préparer avec Claude en direct » (page « MJ · Créer, préparer »).

*État* — Claude a maintenant cinq outils de carte. « Lire l'atlas »
donne toutes les cartes rangées région › lieu › rencontre, avec sur
chacune les scènes épinglées, les PNJ, indices et combats posés, les
lieux de l'histoire et les scènes sans épingle. « Lire une carte en
texte » montre une carte, ou une zone d'une grande carte, en caractères :
des axes numérotés pour nommer une case, le sol de chaque signe (mur,
eau, terrain) et un symbole sur chaque chose posée, avec sa légende.
Les cartes du monde se lisent aussi. « Questions au moteur » répond au
chemin le moins cher à pied et à son coût en mètres, à la ligne de vue et
au couvert, et à ce que voient les joueurs depuis un endroit (leur carte,
le reste dans le brouillard). « Poser sur une carte » épingle une scène
ou un lieu, pose un PNJ, un indice ou un combat « près de » quelque chose
qui a un nom (une étiquette, un décor, une porte, une sortie, un lieu
épinglé, un PNJ déjà posé) ou « sur le chemin » entre deux endroits.
Claude ne donne jamais de coordonnées : Promptus choisit la case, la plus
proche en marchant depuis ce qui est nommé, jamais à travers un mur, ni
sur un mur, ni dans l'eau, libre de tout ce qui est déjà posé ou proposé,
et atteignable depuis l'entrée de la carte. Il dit à Claude où chaque
chose est tombée. Un nom inconnu ou ambigu revient avec la liste de ce
que la carte connaît. Tout passe par le lot de propositions : le MJ voit
les poses en pointillés dans l'Atlas et les garde ou les jette. Enfin,
« Redessiner une carte » envoie des gestes de carte (peindre, poser une
porte…), relus et rejoués une fois si le MJ a touché le même endroit
entre-temps, et proposés eux aussi. Le balayage des routes du MJ reste
vert avec les quatre routes ouvertes au jeton (décision notée dans
`MEMORY.md`). C'est testé côté serveur sur Port-Louis : une PNJ près du
Perroquet Noyé et deux indices sur le chemin du quai, jamais dans un mur
ni dans l'eau ; les outils MCP sont testés contre une API simulée.
*Interprétations à confirmer* : épingler une scène épingle son lieu, et
les autres scènes de ce lieu suivent ; si la scène n'a pas de lieu, ou si
son lieu est déjà épinglé sur une autre carte, Promptus lui crée un lieu
à elle, rangé dans l'ancien, pour ne pas arracher l'autre épingle.
« Atteignable » veut dire atteignable depuis les départs du groupe ou les
sorties de la carte. ⚠️ Sur une carte de région, la mer n'est qu'un
terrain, pas de l'eau au sens des règles : la garde « jamais dans l'eau »
n'y joue pas. **Reste** l'essai réel depuis Claude Code. Valombre et
Oswin n'existent dans aucun des deux mondes : proposition, faire l'essai
sur Port-Louis, Mère Goulven près du Perroquet Noyé et deux indices sur
le chemin du quai (à trancher par Romain).

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

### `ui/adopt-pixel-menu` · doing — reste la relecture de Romain sur son écran

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

**État** — Toute l'interface, MJ et joueur, a pris le dessin du menu
pixel, dans le thème sombre comme dans le clair. Les titres, les
boutons, les étiquettes et les onglets s'écrivent dans une police pixel
servie par l'application elle-même ; la narration, les notes, les fiches
et le texte courant gardent une police lisible. Boutons, panneaux,
champs, onglets et listes de choix ont des coins en escalier et un relief
en aplats : une touche ivoire pour l'action principale, sombre pour les
autres, et une touche s'enfonce d'un pixel quand on appuie. Un curseur de
jeu de rôle (▶) pointe le choix en cours et suit le clavier ; il reste
visible, immobile, quand l'appareil demande moins d'animations. Les
cartes à jouer, les pièces de jeu déjà en pixel art et la grappe de
boutons ronds du combat au téléphone n'ont pas bougé. La page de
référence du design montre le nouveau menu (touches, choix avec curseur,
onglets, champ, tailles de la police). Vérifié à l'écran : la page de
référence et la page de connexion, dans les deux thèmes, sur ordinateur
et au format téléphone, et le curseur au clavier ; le texte sur les
nouvelles touches et onglets garde un contraste d'au moins 4,6:1 dans
les deux thèmes (calculé). Pas vérifié : les
écrans derrière la connexion (MJ et joueur, il faut une passkey) et
l'écran TV ; c'est la relecture qui reste à faire. Choix pris en route :
la police pixel ne descend jamais sous 12 px (lisible sur téléphone), et
les petites étiquettes de caractéristiques des fiches restent dans la
police lisible, faute de place pour la police pixel, qui est large.

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
*Mis à jour par `platform/stream-prep-edits`* : l'écran suit la
préparation en direct — ce que Claude ou le co-MJ viennent de toucher
porte « Modifié », et un bandeau « Claude écrit · sur … » propose
d'annuler sa dernière écriture tant que rien ne l'a recouverte.

*Depuis `gm/prep-on-atlas`* — L’écran de relecture n’est plus une page :
ses cinq onglets et l’atelier du co-MJ vivent dans le tiroir de l’Atlas
(Scènes et actes, Bible, Fiches, Cohérence, Valider). Le graphe y devient
une vue secondaire de la carte.

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
Le co-MJ suit la même règle : il ne propose plus ce que le MJ se
verrait refuser, et une ancienne proposition devenue fausse ne peut
plus être acceptée.
La jauge « prête à jouer » tient compte des cartes propres à la
campagne une fois validées. Vérifié : `cargo test` (règle de
modification, Corsaires et Brasier importés), Vitest de la fiche. Les
pages Médias et Cartes restent. Depuis `campaign/pin-scenes-on-maps`,
on entre aussi dans la fiche par une épingle de l'Atlas : chaque scène
de l'épingle ouvre Préparer sur sa fiche.

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
l'acte 1 réécrit des Corsaires est prêt. Depuis
`campaign/pin-scenes-on-maps`, une scène qui ne se trouve sur aucune
carte (sans lieu, ou aucun lieu épinglé en remontant) est dite à part,
en alerte « scène sans lieu », sans compter dans la jauge ni empêcher
l'acte d'être prêt : un rêve ou un souvenir y ont droit. Le lieu
manquant n'est plus compté parmi les champs d'une scène.

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

### `campaign/pin-scenes-on-maps` · doing — reste l'essai réel sur les deux mondes : en soirée, toucher une épingle lance la scène, et les joueurs voient les épingles révélées sur leur téléphone

**Pourquoi** — Le cœur de l'Atlas : une scène existe quelque part sur la
carte.

**Périmètre** — Une scène porte un lieu (carte et zone, ou case) ;
l'épingle numérotée ouvre l'éditeur de scène existant. Plusieurs scènes
peuvent partager un lieu. Une scène sans lieu (rêve, souvenir, dialogue)
reste permise : le validateur la signale sans bloquer. Côté joueur,
seule la projection serveur montre les épingles révélées. Le format de
campagne gagne ces champs sans rien casser (un fichier ancien s'importe
toujours).

**Fini quand** — Sur les deux mondes, 3 à 5 scènes sont épinglées sur une
carte de région et une carte de lieu ; en soirée, toucher une épingle
lance la scène, et les joueurs voient les épingles révélées sur leur
téléphone.

**Étapes** — Le lieu de l'histoire gagne une ancre facultative (carte +
case ou zone) et une scène désigne son lieu ; la chaîne des parents donne
région › lieu › rencontre → l'étiquette de carte qui nomme une scène est
dérivée de ces ancres ou retirée (bascule nette, décision écrite dans
`docs/map-format.md`) → le validateur signale une scène sans lieu, sans
bloquer → épingles numérotées dessinées sur la carte en hexagones et sur
la carte en cases, qui ouvrent l'éditeur de scène → projection joueur :
seules les épingles révélées → chaque pose écrit une révision du journal.

**Vérification** — Test : un ancien fichier de campagne s'importe sans
ancre ; une ancre vers une carte ou une case qui n'existe pas est
signalée ; la projection joueur ne montre aucune épingle non révélée.
Tests front sur les deux cartes. Essai réel sur les deux mondes.

**Risques** — Deux sources pour la même épingle (l'étiquette de carte et
le lieu) qui divergent : une seule reste. Le format exige les champs
connus : tout nouveau champ est facultatif.

**Origine** — Demande de Romain, 10 octobre 2026 ; planche « Atlas · préparer avec Claude en direct » (page « MJ · Créer, préparer »).

*État* — Livré et vérifié par les tests, pas encore essayé à la main.
Un lieu de la campagne peut être épinglé sur une case d'une carte (du
monde ou de la campagne), et chaque scène apparaît sur l'épingle du
premier lieu épinglé en remontant ses lieux parents : sur la région,
Port-Louis porte toutes les scènes de l'acte 1 ; sur la carte du
quartier, la taverne porte ses trois scènes. Plusieurs scènes partagent
donc une épingle, numérotées dans l'ordre de la campagne (« 1·2·6 »).
Dans l'Atlas, les épingles sont dessinées sur la région en hexagones
comme sur les cartes en cases ; en toucher une liste ses scènes, chacune
ouvre sa fiche dans Préparer, ou la carte de combat de la scène, ou la
fait générer et la rattache à la scène. L'outil « Épingler un lieu »
pose, déplace ou retire une épingle d'une touche sur la case : c'est une
modification de l'histoire comme une autre, inscrite au journal de
préparation (Claude peut faire de même par ses modifications ciblées).
Une scène sans lieu sur une carte (rêve, souvenir) reste permise : le
validateur la signale, la jauge de l'acte la dit à part sans la compter,
et l'Atlas la liste ; une épingle posée sur une carte qui n'existe pas
ou hors de la grille est signalée. En soirée, les épingles de la carte
montrée apparaissent sur la carte du MJ : en toucher une lance sa scène,
ou propose de choisir quand plusieurs scènes s'y trouvent ; sur la carte
en hexagones, l'épingle est en haut de la case, si bien que choisir une
destination de voyage ne change pas. À l'arrivée d'un voyage, la scène
ouverte est la première scène épinglée sur l'hexagone que la table n'a
pas encore jouée. Les joueurs ne voient que les épingles des lieux où la
table est déjà allée, hors du brouillard, avec seulement leur nom :
jamais les scènes ni leurs numéros. Un ancien fichier de campagne
s'importe tel quel. Contenu : nouvelle carte de lieu « Port-Louis, le
quartier bas » (taverne, bureau du port, cour de Kerjean, chantier,
quais, appontement de La Mâchoire), les dix scènes des Corsaires
épinglées (région et quartier), les trois scènes du Brasier (la
Carapace au Reliquaire, comme le disait la carte du système ; l'essaim
à la sortie du Toboggan ; le Cure-Dent épinglé à la sortie du Toboggan
pendant l'acte 1, ce qui y place la scène du Toboggan). Du coup l'Atlas
range région › Port-Louis › quai pour les Corsaires et région › abords
du Toboggan › coursive pour le Brasier. Décisions prises : l'étiquette
de carte ne nomme plus de scène (bascule nette, retirée des cartes
enregistrées) ; une épingle ne range pas les cartes entre elles, seules
les sorties le font ; une épingle est une case, pas une zone ; la
numérotation suit l'ordre des scènes de la campagne. À valider par
Romain : la disposition du quartier de Port-Louis, le lieu
« La sortie du Toboggan », le Reliquaire et le Cure-Dent épinglé pour
l'acte 1. Une campagne importée avant ce lot garde les épingles que ses
cartes donnaient, quand c'est possible sans en déplacer d'autres : une
étiquette qui nommait une scène épingle le lieu de cette scène sur sa
case, si ce lieu n'a pas encore d'épingle et ne contient aucun autre
lieu (épingler un quartier ou tout un système y mettrait toutes les
scènes qu'il contient) ; la première étiquette l'emporte. Sur la carte
du monde des Corsaires, Port-Louis est épinglé là où l'étiquette
« Port-Louis » ouvrait la scène du port de nuit, comme dans le fichier
actuel. ⚠️ Ce qui se perd : une scène sans lieu, ou dont le lieu en
contient d'autres, n'est plus sur aucune carte (le validateur et l'Atlas
la signalent, le MJ épingle un lieu) ; dans le Brasier, la Carapace
était rangée dans le système tout entier, elle n'a donc plus d'épingle
et n'ouvre plus à l'arrivée au Reliquaire ; hors de ces étiquettes,
une campagne importée avant ce lot n'a aucune autre épingle. Pour
l'essai, réimporter le fichier des Corsaires et du Brasier par-dessus
(l'état de la partie est gardé) donne toutes les épingles. Les
étiquettes ont perdu leur scène partout où une carte est gardée : cartes
de la campagne, carte posée sur la table, combat et bataille navale en
cours, journal de préparation. Le co-MJ ne reçoit
pas l'alerte « scène sans lieu », lui qui ne voit pas les cartes :
l'onglet Cohérence renvoie vers l'Atlas plutôt que de lui demander une
correction. Restent : l'essai réel en soirée sur les deux mondes ;
l'écran TV reçoit les épingles visitées mais ne les dessine pas encore ;
une épingle de Claude s'applique encore tout de suite
(`platform/propose-edits-by-claude`) ; l'Atlas n'a pas encore de bouton
pour annuler sa dernière épingle (le journal la garde, Préparer sait
annuler les écritures de Claude seulement).

---

### `campaign/draw-story-thread` · doing — reste l'essai réel : le fil d'un acte des deux mondes affiché sur la région, puis une sortie ajoutée dans l'éditeur de scène qui le redessine sous les yeux du MJ

**Pourquoi** — Voir la campagne d'un coup d'œil : où elle commence, où
elle passe, où elle peut finir.

**Périmètre** — Le fil est calculé depuis les sorties des scènes,
projeté sur les épingles : branches visibles, jamais une ligne unique.
Liste des scènes à côté, synchronisée avec la carte (cliquer l'une zoome
l'autre). Le validateur signale deux scènes reliées dont les lieux ne
sont reliés par aucun chemin.

**Fini quand** — Le fil d'un acte des deux mondes s'affiche sur la
région, une sortie ajoutée dans l'éditeur de scène redessine le fil
sans autre geste.

**Étapes** — Calcul pur du fil dans le crate partagé (sorties des scènes
→ segments entre ancres, remontés au niveau de carte affiché par la
chaîne des parents) → tracé sur les deux cartes, branches comprises →
liste des scènes synchronisée avec la carte → alerte du validateur
« deux scènes reliées sans chemin entre leurs lieux » → redessin à chaque
révision reçue.

**Vérification** — Tests du calcul : une sortie ajoutée ajoute un
segment, une branche reste deux segments, une scène sans lieu ne casse
rien ; test du validateur sur un chemin bloqué ; test front du redessin
après une révision.

**Risques** — Un fil illisible quand beaucoup de scènes partagent un
lieu : regrouper par lieu. Un calcul de chemin coûteux sur une grande
région : le faire à la demande, pas à chaque image.

*État* — Dans l'Atlas, les épingles de scène sont reliées par le fil de
la campagne, sur la région en hexagones comme sur une carte de lieu : une
flèche ivoire pleine par couple de lieux qu'une sortie relie, dans chaque
sens. Une scène qui mène à deux lieux dessine donc deux branches, et un
retour se dessine à côté de l'aller au lieu de le recouvrir. Les scènes
d'un même lieu partagent une épingle : les sorties entre elles restent
dans l'épingle, sans trait. Une scène sans lieu ou hors de la carte ne
casse rien, elle n'est simplement pas tracée. À côté de la carte, la
liste « Le fil de la campagne » donne les scènes posées sur la carte,
dans l'ordre, avec « départ » (rien de son acte n'y mène) et « fin
possible » (aucune sortie ne reste dans son acte), et un filtre par acte.
Toucher une scène de la liste allume ses flèches, choisit son épingle et
amène la carte de lieu jusqu'à elle (🔴 non mesuré : ce défilement n'est
vérifié par aucun test, l'environnement de test n'ayant pas de mise en
page) ; toucher une épingle marque ses scènes dans la liste. Un acte
choisi qui n'a aucune scène sur la carte ouverte revient à « tous les
actes ». Le fil suit le calque « Histoire ». Il est
recalculé à chaque lecture de l'Atlas et rien ne le stocke : une sortie
ajoutée dans l'éditeur de scène, par Claude ou par le co-MJ redessine le
fil dès que la révision arrive à l'écran, sans autre geste. Les joueurs
et l'écran TV ne le reçoivent jamais.
Le validateur avertit (jamais ne refuse) quand une sortie relie deux
scènes qu'aucun chemin ne joint, et l'Atlas le dit en nommant la scène.
**Hypothèse retenue** pour « chemin » : deux scènes sont jointes si
leurs lieux ont un lieu commun en remontant les parents (la taverne et
le port d'une même ville), ou si un lieu de chacune est posé sur la même
carte sans qu'un mur les sépare (une porte laisse passer, les hexagones
comptent leurs six voisins), ou sur deux cartes reliées par des sorties,
dans un sens ou dans l'autre. Le chemin n'est calculé que pour cet
avertissement, une passe par carte, pas à chaque image. Claude ne tente
pas de le réparer. Aucun des deux mondes livrés ne déclenche
l'avertissement. Tout cela est testé côté moteur, serveur et interface.
⚠️ Sur la région des Corsaires, l'acte 1 tient presque entier dans
l'épingle de Port-Louis : la région n'y montre qu'une flèche, vers
Belle-Île. Les branches de l'acte (la nuit au port vers le marché noir,
le chantier, la capitainerie et le quai) se voient sur la carte du
quartier de Port-Louis. Reste l'essai réel sur les deux mondes.

**Origine** — Demande de Romain, 10 octobre 2026 ; planche « Atlas · préparer avec Claude en direct » (page « MJ · Créer, préparer »).

---

### `campaign/place-cast-and-clues` · doing — reste l'essai en soirée : Romain place Oswin et deux indices au cimetière, une rencontre à l'entrée de la crypte, et les retrouve au même endroit, révélables un par un

**Pourquoi** — Écrire une scène, c'est aussi placer qui s'y trouve et ce
qu'on y trouve.

**Périmètre** — PNJ (sprites), indices (losanges) et rencontres posés
sur la carte, sur le calque du MJ, éventuellement propres à une scène ;
révélés pendant la soirée comme le reste. Calques Histoire, Personnages,
Combats, Indices qu'on allume et éteint. Une rencontre pointe vers son
vrai plan de combat, adversaires déjà placés.

**Fini quand** — Romain place Oswin et deux indices au cimetière, une
rencontre à l'entrée de la crypte, et les retrouve en soirée au même
endroit, révélables un par un.

**Étapes** — Positions de préparation dans l'histoire (PNJ, indice,
rencontre : carte + case, éventuellement propres à une scène), champs
facultatifs → pose, déplacement et retrait sur le calque du MJ, chacun
une révision → calques allumables → pont vers la soirée : quand la soirée
ouvre la carte, ces positions deviennent des pions cachés ; la
révélation passe par les gestes existants (révéler un PNJ, un indice) →
une rencontre ouvre son plan de combat, adversaires placés.

**Vérification** — Tests : un ancien fichier s'importe ; ouvrir la carte
en soirée crée les pions au bon endroit, cachés ; la projection joueur
ne les montre qu'une fois révélés ; une position sur un mur est
signalée. Essai réel en soirée.

**Risques** — Deux copies d'une même position (préparation et soirée) :
la préparation ne change jamais ce qui est déjà sur la table d'une
soirée ouverte. Une fuite d'un PNJ non rencontré : la projection reste
le seul passage.

*État* — Dans l'Atlas, le MJ pose sur n'importe quelle carte (région en
hexagones ou carte de lieu) un PNJ (pion rond à son initiale), un indice
(losange) ou le combat d'une scène (carré noir barré). Il les déplace et
les retire du panneau, et chaque geste est une ligne du journal de
préparation, comme une épingle. Un PNJ peut être posé pour toutes les
scènes jouées sur la carte, ou pour une seule. Il apparaît alors pâli.
Quatre calques de l'histoire s'allument et s'éteignent à part des
calques de la carte : Histoire (les épingles de scène), Personnages,
Combats, Indices. Un combat dit combien d'adversaires son plan de combat
place déjà (« 2 / 3 », en rouge s'il en manque, ce qui empêcherait de le
lancer) et ouvre ce plan d'un geste. Un PNJ, un indice ou un combat posé
dans un mur est signalé, jamais refusé. Hors de la carte ou sur une
carte inconnue, c'est une erreur. Claude ne tente jamais de réparer ces
cases.
En soirée, montrer la carte y pose les PNJ prévus, cachés, à leur case.
Ceux d'une scène précise arrivent quand cette scène s'ouvre pour la
première fois. Une carte remontrée après une autre repart de zéro, mais
un PNJ déjà rencontré y revient visible. Le geste « Les joueurs rencontrent Oswin » le fait
apparaître aux joueurs avec son nom. Les indices sont des losanges pâles
tant qu'ils ne sont pas trouvés : un toucher propose « Les joueurs le
trouvent ». Une fois trouvé, l'indice s'affiche sur le téléphone à sa
case, avec son texte (jamais son identifiant). Toucher le combat le
lance. Ce qui est déjà sur la table d'une soirée ouverte ne bouge
jamais quand la préparation change. Tout cela est testé côté serveur et
interface, et un ancien fichier de campagne s'importe sans changement.
**Hypothèses retenues** : les positions vivent dans l'histoire, à côté
de ce qu'elles placent (le PNJ, l'indice, le combat de la scène), pas
sur la carte. La scène d'un indice est la sienne. Un indice n'est pas un
pion : on le révèle par le geste d'indice existant. La rencontre posée
sur une carte de lieu ouvre le plan de combat déjà rattaché à la scène.
**Pas fait** : placer d'un geste les adversaires qui manquent sur le
plan. Le MJ les pose dans l'éditeur de carte, le compteur lui dit
combien il en manque. L'écran TV ne dessine pas encore les indices
trouvés. Un PNJ posé sur une région en hexagones reste un repère de
préparation : la soirée n'y pose pas de pion (le groupe y voyage en un
seul pion). ⚠️ « Montrer aux joueurs » sur un pion caché affiche son nom
même si les joueurs ne l'ont pas rencontré (comportement antérieur, pour
tout pion de PNJ) : pour une vraie rencontre, utiliser « Les joueurs
rencontrent ». Masquer le nom tant que le PNJ n'est pas rencontré est à
trancher.
🔴 Aucun cimetière, aucune crypte et aucun Oswin n'existent dans le
contenu des deux mondes. Pour l'essai, il faut soit créer ces cartes et
ce PNJ, soit jouer la même chose sur Port-Louis, par exemple Mère
Goulven au Perroquet Noyé. Le choix revient à Romain.

**Origine** — Demande de Romain, 10 octobre 2026 ; planche « Atlas · préparer avec Claude en direct » (page « MJ · Créer, préparer »).

---

### `campaign/prepare-with-claude` · doing — reste l'essai réel : Romain prépare sa prochaine soirée avec son Claude, de la première scène à « Jouable »

**Pourquoi** — La création actuelle (`/campagnes/nouvelle` puis
`/generer`) demande un formulaire avant toute idée, fait saisir l'idée
deux fois, demande le budget IA trop tôt et aboutit à une campagne
vide. Romain prépare désormais en parlant à son Claude, et le MCP
Promptus a fait ses preuves : la préparation devient une conversation,
l'app ne sert plus qu'à regarder et à valider.

**Périmètre** — Un skill Claude « préparer ma prochaine soirée », livré
avec `mcp/` et utilisable depuis Claude Desktop comme depuis Claude
Code, qui suit les moments de la planche « Créer · la campagne de
Romain (refonte) » : (1) départ d'une idée, d'un fichier ou d'une
campagne existante ; (2) Romain raconte une scène, d'ouverture ou de
fin, et Claude en tire l'univers et le ton, à confirmer ; (3) Claude
propose autour d'elle le lieu, deux PNJ, trois indices et une tension
avec son horloge de 6 étapes, chacun en proposition gardée ou jetée
par Romain ; (4) les indices gardés deviennent des scènes proposées, à
rebours si l'on est parti de la fin ; (5) les règles au premier
combat ; (6) « Jouable » selon `get_readiness`, puis seulement le
titre et l'accroche pour inviter la table. Les outils MCP qui manquent
pour ces cartes (tension et horloge, scène à rebours) sont ajoutés,
toujours en propositions (`platform/propose-edits-by-claude`). Les
pages `/campagnes/nouvelle` et `/generer` sont retirées ; la variante
fichier reste.

**Fini quand** — Romain prépare sa prochaine soirée en conversation
avec Claude, de la première scène à « Jouable », sans ouvrir de
formulaire dans l'app.

**Origine** — Demande de Romain, 10 et 11 octobre 2026 ; planche « Créer
· la campagne de Romain (refonte) », page « MJ · Créer » du canevas ;
`docs/claude-mcp.md`.

*État* — Le skill « Préparer ma prochaine soirée » est dans
`mcp/skills/preparer-soiree/` ; le serveur MCP le sert aussi comme
invite (`preparer_soiree`), donc Claude Desktop et Claude Code l'ont sans
rien installer. Il suit les six moments : départ (idée, campagne
existante ou fichier), une scène racontée puis l'univers et le ton à
confirmer, autour d'elle un lieu, deux PNJ, trois indices et une tension
avec son horloge, les indices gardés devenus scènes (à rebours depuis la
fin si l'on est parti d'elle), les règles au premier combat, « Jouable »
puis le titre et l'accroche. Nouveaux outils MCP : `start_campaign`
(une scène et l'univers confirmé deviennent une campagne, sans
formulaire ; les règles sont provisoires jusqu'au premier combat),
`propose_tension` (un front et son horloge de 6 étapes, proposé),
`propose_scene` (une scène reliée en avant ou à rebours, éventuellement
la nouvelle ouverture, avec ses indices, proposée élément par élément),
`list_rule_systems` ; `get_readiness` dit désormais « Jouable » ou ce
qui manque. Les pages `/campagnes/nouvelle` et `/generer` sont retirées :
la carte « Préparer avec Claude » de l'accueil mène à l'accès pour
Claude, l'import d'un fichier YAML reste. Essayé de bout en bout en
local par un vrai client MCP : campagne née d'une scène de fin, tension
et scène à rebours proposées, lues dans `list_proposals`. Testé côté MCP
et serveur. **Hypothèses retenues** : la première scène crée la campagne
directement (il n'y a rien sur quoi proposer) ; la campagne joue le
premier système de règles connu tant que le MJ n'a pas choisi ; les
routes serveur de création et de génération restent, sans écran, pour
l'évaluation et les tests. **Reste** l'essai réel avec Romain.

### `campaign/import-pixellab-assets` · doing — reste l'essai réel avec un vrai abonnement PixelLab (`media/choose-pixellab-plan`)

**Pourquoi** — Ce que Claude prépare doit devenir ce que la table verra
(le lieu en HD-2D, les PNJ en pixel art) sans que Romain dessine ni
copie de fichier.

**Périmètre** — Un outil MCP Promptus qui reçoit une image PixelLab (son
adresse) et l'attache en proposition : élément d'un thème (texture vue
de face d'une matière ou d'une face d'accessoire, façade, accessoire
plat), personnage ou PNJ avec ses directions et animations. Le serveur
télécharge l'image lui-même et la range dans `media_assets`, en
attente de validation comme les autres images. Le skill de
`campaign/prepare-with-claude` enchaîne les deux MCP : pour un lieu ou
un PNJ gardé, Claude dresse la liste de ce qui manque au thème, annonce
le coût en générations, génère chez PixelLab après l'accord de Romain,
ramène les couleurs sur la palette du thème et dépose le résultat. Ce
qui existe déjà dans le thème est réutilisé, jamais regénéré. Règle de
forme : `MEMORY.md` §2.

**Fini quand** — Pour la scène d'ouverture de la prochaine soirée, le
lieu et ses PNJ arrivent dans Promptus en éléments PixelLab, validés
par Romain, avec le coût total annoncé avant chaque lot.

**Origine** — Priorité de Romain, 11 octobre 2026 ; test PixelLab du même
jour. Dépend de `media/choose-pixellab-plan`.

*État* — Deux outils MCP. `get_theme_needs` liste, pour une carte, ce que
le rendu HD-2D attend du thème : la texture de chaque sol utilisé, des
murs, des deux faces (côté, dessus) de chaque accessoire en volume,
l'image plate de chaque accessoire fin, et le personnage de chaque PNJ
posé sur la carte (ou d'un PNJ donné) ; chacun manque, en attente ou
gardé, avec son coût en générations (1 par image, 6 par personnage :
une estimation) et la palette du thème. `import_pixellab_asset` dépose
une image par son adresse : le serveur la télécharge lui-même, depuis
PixelLab seulement (`PIXELLAB_HOSTS`, `pixellab.ai` par défaut) ou une
adresse `data:`, vérifie que c'est un PNG raisonnable, ramène les
couleurs d'un élément sur la palette du thème (un personnage garde les
siennes) et la range dans `media_assets`, en attente : le MJ la garde ou
la jette comme les autres images. Un personnage arrive avec toutes ses
images (directions et animations). Un emplacement qui a déjà une image
en attente ou gardée est refusé (`ELEMENT_EXISTS`) : rien ne se
regénère, sauf si le MJ demande de remplacer. Le skill enchaîne les deux
MCP : liste, coût annoncé, accord du MJ, génération, dépôt. Les joueurs
voient un élément une fois gardé, un personnage une fois le PNJ
rencontré. Testé côté serveur (téléchargement refusé hors PixelLab,
palette, doublon, personnage et ses images, jeton qui ne décide pas) et
MCP. **Hypothèses retenues** : les emplacements d'un thème sont
`<tileset>/material/<id>`, `<tileset>/wall`, `<tileset>/prop/<id>/side`
et `/top` pour un volume, `<tileset>/prop/<id>` pour un accessoire fin ;
pas encore d'emplacement de façade, tant que la scène ne dessine pas de
bâtiment (rien ne se génère que le MJ ne puisse ni voir ni valider) ; qui est volume ou plat se décide par la forme de
l'accessoire dans le thème (caisse, tonneau, trappe… volume ; cordage,
mât, échelle… plat). Les adversaires des combats posés sur la carte
comptent aussi parmi les personnages à générer. Les images d'un personnage
se téléchargent huit à la fois. Toutes les images PixelLab en attente se
gardent ou se jettent aussi dans le tiroir « Médias » de l'Atlas, quelle
que soit leur carte. **Pas fait** : le rendu des animations des
personnages (`hd2d/show-pixellab-characters`), les façades dans la
scène. **Reste** l'essai réel avec l'abonnement PixelLab : les adresses
que renvoie le MCP de PixelLab sont-elles bien sur `pixellab.ai` ? Sinon,
ajouter leur hôte à `PIXELLAB_HOSTS`.

### `campaign/watch-prep-in-hd2d` · doing — reste l'essai réel à côté de Claude, sur la TV (attend `hd2d/render-board-in-three`) et 60 images/s sur un iPad

**Pourquoi** — L'éditeur devient un visualiseur : Romain édite la
campagne avec Claude et voit le rendu.

**Périmètre** — L'Atlas montre la scène en préparation dans le rendu
HD-2D (`hd2d/render-board-in-three`), mis à jour en direct à chaque
modification de Claude (`platform/stream-prep-edits`) : lieux,
éléments et PNJ posés à leur place, propositions en attente visibles
et distinguées de ce qui est validé, garder ou jeter d'un geste. Ce
qui manque encore (élément pas généré, PNJ sans personnage) est
signalé plutôt que caché. L'édition à la main se réduit aux retouches.

**Fini quand** — Pendant que Romain prépare avec Claude, l'Atlas ouvert
à côté montre la scène d'ouverture en HD-2D changer sous ses yeux,
comme le prototype du quai (`docs/design/hd2d/quai.html`), sur
ordinateur et sur la TV.

**Origine** — Romain, 11 octobre 2026 : « Je vois bien l'éditeur plus
devenir un visualisateur. J'édite la campagne avec Claude et je vois le
rendu. »

*État* — L'Atlas a un sélecteur « Plan / HD-2D » (`?rendu=hd2d`). En
HD-2D, la carte d'un lieu ou d'une rencontre devient une scène Three.js
construite depuis sa grille : sols texturés, murs en volumes, eau plus
basse, accessoires en volume (boîtes texturées) ou plats selon leur
forme, lumières de la carte qui vacillent, heure et brume de son
ambiance, ombres. Les éléments PixelLab du thème y sont posés ; ce qui
manque est peint avec les couleurs du thème et listé à côté (« Ce qui
manque encore », avec le coût en générations), jamais caché ; une image
PixelLab en attente pulse dans la scène et se garde ou se jette d'un
toucher dans le panneau. Les épingles, PNJ (avec leur personnage
PixelLab s'il existe), indices et combats sont des pièces debout ; ce
que Claude propose flotte translucide, en pointillés, marqué ▶ ; toucher
une proposition ouvre son lot et une barre « Garder / Jeter » au-dessus
de la scène. La scène se reconstruit à chaque révision et à chaque image
reçue, sans recharger, et garde le point de vue du MJ. Vérifié dans un
vrai Chromium sur le quai de Port-Louis : un PNJ placé par le jeton de
Claude apparaît sous les yeux, translucide et marqué ▶. Sans WebGL,
l'Atlas le dit et le plan reste à un toucher. Testé (modèle de scène,
Atlas). **Hypothèses retenues** : une région en hexagones reste en plan ;
la vue est celle du MJ (les pièces cachées aux joueurs y sont) ; le
rendu vit dans `front/src/features/hd2d/`, que
`hd2d/render-board-in-three` reprendra pour le plateau, la TV et les
téléphones. **Pas fait** : la TV (elle ne montre que la soirée : il faut
`hd2d/render-board-in-three`), le flou de profondeur et le halo du
prototype, les façades. **Reste** l'essai réel pendant une préparation
avec Claude, et la tenue sur un iPad.

### `campaign/write-story-by-map-scale` · doing — reste l'essai réel de Romain

**Pourquoi** — Sur la page de la campagne, l'Atlas est une carte-bouton
parmi d'autres (« Mener la soirée », « Inviter la table », « Règles »),
à côté d'un formulaire de réglages. Or l'histoire s'écrit à chaque
échelle de carte : l'Atlas n'est pas une rubrique, c'est le point de
départ de la préparation.

**Périmètre** — Chaque échelle de carte porte la part de l'histoire qui
lui correspond, et s'écrit là :

- **L'échelle la plus large** (`world` dans le code : la région en
  hexagones pour les Corsaires) porte **l'idée de la campagne** — titre,
  univers, idée (MJ seul), accroche (lue par les joueurs), ton, thèmes —
  puis **les actes et les fronts**. Un panneau « L'idée » dans le panneau
  latéral de l'Atlas les montre et les édite, avec les réglages
  techniques repliés en dessous (nombre de joueurs, budget IA).
- **Le lieu** porte ses scènes et ses PNJ ; **la rencontre**, son
  combat, ses indices et son butin — c'est déjà le cas, inchangé.
- **Entrer dans une campagne, c'est ouvrir l'Atlas** : `/campagnes/:id`
  mène à l'Atlas sur sa carte la plus large. La page de réglages
  disparaît ; ses actions passent dans l'en-tête de l'Atlas (« Mener la
  soirée », « Inviter la table », « Règles », « Archiver » /
  « Désarchiver ») et son bandeau d'état (en préparation, archivée)
  au-dessus de la carte.
- **Une campagne sans carte** ouvre l'Atlas sur le panneau « L'idée »
  seul, avec le chemin vers ses cartes (onglet « Cartes » du tiroir).
- L'onglet « Bible » du tiroir garde ce qui n'a pas de carte (vérités,
  secrets, direction artistique) ; un champ édité dans « L'idée » n'est
  pas édité une seconde fois ailleurs.
- Les modifications de Claude par le MCP (`apply_story_edits` sur
  `bible`, `campaign`, un acte, un front) apparaissent dans « L'idée »
  sans recharger, en proposition quand Claude n'a pas la main, comme
  le reste de l'Atlas.

**Fini quand** — Romain ouvre les Corsaires depuis l'accueil, arrive sur
l'Atlas, retravaille le pitch, l'accroche et l'acte 1 sur la carte la
plus large, descend jusqu'au quai de Port-Louis, lance la soirée depuis
l'en-tête — sans repasser par une page de réglages. Tests à jour,
`bun run lint` et `bun run test` passent.

**Origine** — Romain, 11 octobre 2026, en reprenant l'acte 1 des
Corsaires : « Atlas devrait pas être une catégorie, mais un point de
départ. On écrit l'histoire aux différents niveaux de granularité de la
map. La map globale est l'idée, le pitch, le titre, l'univers. »

*État* — Codé et testé sur la branche `claude/atlas-first`. `/campagnes/:id`
est l'Atlas lui-même (même composant que `/campagnes/:id/atlas`, qui
marche toujours) : la page de réglages `CampaignPage` est supprimée, et
tous les liens existants (accueil, liste des campagnes, retours des
autres pages) arrivent donc sur l'Atlas. Il s'ouvre sur la première
racine à l'échelle la plus large que l'atlas contient. L'en-tête porte
le titre de la campagne, son état, « Mener la soirée », « Inviter la
table », « Règles » et « Archiver » (avec confirmation) / « Désarchiver » ;
le bandeau d'état est sous l'en-tête, au-dessus de la carte. Le panneau
« L'idée » (`atlas/IdeaSection.tsx`) édite titre, univers, idée,
accroche, ton et thèmes, puis les actes (titre, en bref, ouverture,
clôture) et les fronts (nom, menace, description, horloge), avec les
réglages techniques repliés (joueurs, budget IA, système de règles en
lecture). L'onglet « Bible » du tiroir ne garde que vérités, secrets et
direction artistique. Les éléments de Claude en attente sur `campaign`,
`bible`, un acte ou un front s'affichent en pointillés sous le champ
concerné, gardés ou jetés sur place ; ce que quelqu'un d'autre vient de
changer porte « Modifié » ; la campagne est relue à chaque révision
(`prep`, `proposals`), sans écraser ce que le MJ est en train de taper.
**Hypothèses retenues** : « l'échelle la plus large » est celle de la
carte la plus large *présente* dans l'atlas (une campagne sans région
mais avec des lieux écrit son idée sur ses lieux racines) ; le titre,
l'univers, l'idée, l'accroche, les joueurs et le budget passent toujours
par `PUT …/settings`, le ton et les thèmes par des modifications de la
bible (deux lignes du journal quand les deux changent) ; une horloge de
front s'édite une étape par ligne, chaque étape gardant la description
de celle qu'elle remplace au même rang ; on ajoute un acte ou un front
mais on n'en retire pas depuis « L'idée » (les scènes d'un acte
retiré resteraient orphelines). **Reste** l'essai réel de Romain sur
les Corsaires (pitch, accroche, acte 1, descente jusqu'au quai, soirée
lancée depuis l'en-tête).


### `campaign/tidy-atlas-layout` · doing — reste l'essai réel de Romain

**Pourquoi** — Depuis `campaign/write-story-by-map-scale`, le panneau
latéral de l'Atlas empile « L'idée » puis le panneau de la carte : il
faut défiler longtemps pour atteindre l'un ou l'autre, et 320 px sont
étroits pour écrire un pitch. Au-dessus de la carte, quatre rangées
(en-tête et actions, bandeau d'état, fil d'Ariane et bascules, deux
rangées de calques) mangent la hauteur avant le premier hexagone.

**Périmètre**

- **Panneau latéral en onglets** : « L'idée » (à l'échelle la plus
  large seulement), « La carte » (fil de l'histoire, personnages,
  HD-2D, ce que le panneau montre aujourd'hui) et « Claude » (ses lots,
  avec le nombre en attente en pastille). Un seul onglet visible à la
  fois ; le choix est dans l'URL (`?panneau=`). À l'échelle la plus
  large on arrive sur « L'idée », ailleurs sur « La carte ». Un lot de
  Claude qui arrive allume la pastille sans changer d'onglet.
- **« L'idée » prend de la largeur** : quand l'onglet est ouvert, le
  panneau s'élargit (la carte rétrécit), assez pour écrire un pitch.
- **Gagner de la hauteur au-dessus de la carte** :
  - titre, état et actions sur une seule rangée ;
  - le bandeau « En préparation · rien n'est montré… » devient le texte
    de la puce d'état (au survol / au toucher), plus une rangée ;
  - « Vue MJ / Vue joueurs » et « Plan / HD-2D » en deux sélecteurs
    segmentés compacts sur la rangée du fil d'Ariane ;
  - les calques de l'histoire et ceux de la carte sur une seule rangée
    (ou repliés dans un menu « Calques »).
- Sur tablette, le panneau replié reste replié ; les onglets sont en
  haut du panneau quand il s'ouvre.

**Fini quand** — Sur l'écran de Romain (ordinateur portable), la carte
de la région des Corsaires commence dans le premier tiers de l'écran,
on passe de « L'idée » à « La carte » à « Claude » d'un clic sans
défiler, et le pitch s'écrit sans être à l'étroit. Tests à jour, lint et
tests passent.

**Origine** — Romain, 11 octobre 2026, au premier essai de l'Atlas
comme point d'entrée : « C'est étrange d'avoir l'idée puis en dessous
la carte. Je préférerais que ce soit un switch de tabs. Et je pense en
termes d'UX qu'on peut améliorer et gagner de la place. »

*État* — Le panneau latéral de l'Atlas est en onglets ARIA
(`atlas/PanelTabs.tsx` : `tablist` / `tab` / `tabpanel`, flèches,
Début et Fin) : « L'idée » (à l'échelle la plus large seulement),
« La carte » (HD-2D, fil, épingles, personnages, « On y entre ») et
« Claude » (ses lots), un seul visible ; le choix est dans
`?panneau=idee|carte|claude`. À l'échelle la plus large on arrive sur
« L'idée », ailleurs sur « La carte ». L'onglet « Claude » porte le
nombre de lots en attente en pastille (nom accessible « Claude · 2 lots
en attente ») ; un lot qui arrive la met à jour sans changer d'onglet.
Quand « L'idée » est ouverte, le panneau passe de 320 à 480 px. Au-dessus
de la carte : titre, puce d'état et actions sur une rangée ; le bandeau
« En préparation · … » / « Archivée le … » est devenu la description de
la puce (bulle au survol, au focus clavier ou au toucher, `aria-describedby`) ;
« Vue MJ / Vue joueurs » et « Plan / HD-2D » sont deux sélecteurs
segmentés compacts sur la rangée du fil d'Ariane ; calques de l'histoire
et calques de la carte partagent une rangée. La « Vue joueurs » garde son
panneau tel quel, sans onglets ; sans carte, « L'idée », « Pas encore de
carte » et Claude restent côte à côte. **Hypothèses retenues** : le choix
d'onglet est collant — choisir « La carte » sur la région le garde de
carte en carte et au retour ; un choix « L'idée » retombe sur « La carte »
dans un lieu et revient en ressortant ; toucher un lot de Claude sur la
carte ou une proposition en pointillés dans « L'idée » ouvre l'onglet
« Claude » sur ce lot, toucher une épingle ou une pièce ouvre « La
carte » (sa fiche y est) ; « Le détail » du bandeau HD-2D ouvre le
panneau sur « Claude » ; les calques tiennent sur une rangée sans menu
« Calques » (quatre calques d'histoire et deux ou trois de carte tiennent
sur un portable ; la rangée passe à la ligne au besoin) ; la pastille
compte les lots en attente, pas leurs éléments. Sur tablette, « Replier »
est au bout de la rangée d'onglets et le panneau replié le reste. **Reste**
l'essai réel de Romain sur son portable (la région des Corsaires dans le
premier tiers de l'écran, le pitch écrit à l'aise).

### `campaign/order-maps-by-story` · doing — reste l'essai réel de Romain

**Pourquoi** — Romain veut écrire l'histoire en suivant les cartes dans
l'ordre où les joueurs les traversent. Aujourd'hui la bande de cartes de
l'Atlas suit l'emboîtement (région › lieu › rencontre), rien ne dit
dans quel ordre l'histoire passe par elles, et une épingle peut porter
toutes les scènes d'un acte (Port-Louis porte les 9 scènes de l'acte 1).

**Périmètre**

- **Une frise par acte**, dans l'ordre des actes : un acte est une
  soirée (« Soir 1 · L'Offre qu'on ne refuse pas »), aucun champ à
  ajouter ; puis les cartes que ses scènes traversent, dans l'ordre du
  chemin entre les scènes (départ de la campagne ou ouverture de l'acte,
  puis les sorties). Des scènes au choix (des sorties parallèles)
  forment un embranchement : leurs cartes sont empilées sur la même
  case (« marché · chantier · bureau »).
- **La carte d'une scène** est la carte la plus fine qui porte son lieu
  (le lieu épinglé, ou en remontant ses parents). Une scène dont le lieu
  n'a pas de carte à lui fait apparaître une **case « à créer »** au nom
  du lieu (« Le Perroquet Noyé — pas de carte ») ; la toucher mène à la
  création de la carte de ce lieu (onglet « Cartes » du tiroir, lieu
  prérempli), ou la demande à Claude.
- **Toucher une case** ouvre sa carte dans l'Atlas, avec les scènes de
  l'acte qui s'y passent mises en avant ; la case de la carte ouverte
  est marquée dans la frise.
- La frise remplace la bande de cartes en bas de l'Atlas en vue MJ ;
  les cartes qu'aucune scène ne traverse restent accessibles à la fin
  (« Hors de l'histoire »). Un acte sans scène montre une case vide
  « Écrire l'acte ».
- **Claude** : `get_atlas` (MCP) renvoie la même frise par acte, cartes
  manquantes comprises, pour qu'il sache quelles cartes créer.
- Calculé côté serveur depuis le graphe (un seul ordre partout, Atlas
  et MCP), mis à jour en direct comme le reste de l'Atlas.

**Fini quand** — Sur les Corsaires, l'Atlas montre l'acte 1 en frise
(Perroquet Noyé à créer → port de nuit → marché · chantier · bureau →
quai → La Mâchoire) puis l'acte 2 ; Romain touche une case et arrive sur
la bonne carte ; il crée la carte du Perroquet Noyé depuis sa case et la
frise la prend aussitôt. Tests à jour, lint et tests passent.

**Origine** — Romain, 11 octobre 2026, en voulant passer les Corsaires
à 4 actes en 4 soirées : « Il me manque les cartes dans l'ordre
chronologique. C'est surtout comme ça que je veux travailler
l'histoire. »

*État* — La frise est calculée une fois, côté serveur, par une fonction
pure testée (`shared/src/story/frieze.rs`), à chaque lecture de l'atlas :
`GET …/atlas` la renvoie (`frieze` : par acte `evening`, `title`, `slots`
de `stops`, puis `outside`), et `get_atlas` la donne à Claude (`frise`,
une case par étape, `carte_à_créer_pour` pour une carte manquante,
`hors_de_l_histoire`). Dans l'Atlas, en vue MJ, elle remplace la bande
de cartes (la vue joueurs garde la bande) : « Soir N · titre de l'acte »,
puis les cases reliées par des flèches, un embranchement empilé dans une
case (« Au choix »), une carte à créer en pointillés (« Le Perroquet
Noyé — pas de carte »), un acte vide « Écrire l'acte » (ouvre « Scènes et
actes »), enfin « Hors de l'histoire ». Elle défile de côté sur téléphone
et tablette, et suit le direct comme le reste de l'Atlas. Toucher une
case ouvre sa carte, marque la case et règle le fil sur cet acte ;
toucher une case à créer ouvre le tiroir « Cartes » (`?lieu=`) sur « La
carte de « … » » : nom prérempli, vierge ou copie d'une carte du monde ;
créée, le lieu y est épinglé au milieu, l'Atlas l'ouvre et la frise la
prend. **Règles retenues** : la carte d'une scène est d'abord la carte où
elle se joue (`map` de la scène) si l'atlas l'a ; sinon la plus fine des
cartes où son lieu ou un de ses parents est épinglé (à échelle égale, le
lieu le plus proche) ; elle est **à créer**, au nom du lieu de la scène,
quand rien de sa chaîne n'est épinglé, ou quand la seule carte est une
région et que c'est un parent qui y est épinglé (la taverne dans
Port-Louis n'est pas dessinée par l'hexagone de Port-Louis) ; une scène
sans lieu ni carte n'a pas de case. Le chemin part de `start_node` s'il
est dans l'acte, sinon des scènes de l'acte où aucune autre ne mène
(aucune : la première) ; d'une scène à une seule suite on passe à la
suivante ; d'une scène à plusieurs suites, tout ce qui précède le point
où tous les chemins se rejoignent (la scène par laquelle passe tout
chemin vers une fin d'acte) forme l'embranchement, puis le chemin reprend
là ; des chemins qui ne se rejoignent pas mettent tout le reste dans
l'embranchement. Les sorties vers un autre acte ou vers une scène déjà
parcourue ne sont pas suivies (pas de boucle) ; les scènes jamais
atteintes reprennent à la fin. Deux cases de suite sur la même carte
seule n'en font qu'une. La « mise en avant » des scènes de l'acte est le
filtre d'acte du fil (« Acte du fil »), déjà là. **Ce que donnent les
Corsaires** : en production (seul Port-Louis épinglé, sur la région),
Soir 1 = Perroquet Noyé (à créer) → port de nuit (à créer) → marché ·
chantier · bureau · Perroquet Noyé (Anaïs), tous à créer → Le quai de
Port-Louis (la carte de la bagarre) → La Mâchoire (à créer) ; Soir 2 =
L'abordage du Greyhound (la carte de la scène ; Belle-Île n'est épinglée
nulle part, « Au large de Belle-Île » n'est liée à aucune scène et reste
hors de l'histoire, avec la région, les rues et Le Palais). Écarts avec
l'attente : « Ramener Anaïs » est dans l'embranchement (elle part du
chantier et rejoint le quai) ; le port de nuit et La Mâchoire sont aussi
à créer. Dans le fichier du dépôt, où chaque lieu est épinglé sur les
rues, Soir 1 = les rues → le quai → les rues. **Limite** : le tiroir
« Cartes » ne connaissait pas les lieux ; la création « pour un lieu »
y est ajoutée, et pour la demander à Claude il suffit de lui parler :
`get_atlas` lui liste les cartes à créer. **Reste** l'essai réel de
Romain sur les Corsaires (toucher une case, créer la carte du Perroquet
Noyé depuis sa case).

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

*Depuis `gm/prep-on-atlas`* — Appliquer le brouillon ouvre l’Atlas, et la
scène d’ouverture y est déjà posée sur la région du monde (même ligne du
journal que le brouillon).


*Depuis `campaign/prepare-with-claude`* — La page `/campagnes/:id/generer`
et son bouton sont retirés : une campagne se prépare désormais en
conversation avec Claude. La génération reste côté serveur (routes,
évaluation `bun run eval`), sans écran. À trancher par Romain : garder
ce ticket pour l'évaluation seule, ou l'archiver.
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

*Depuis `gm/prep-on-atlas`* — Les images et vidéos se préparent dans le
tiroir de l’Atlas (onglet Médias) ; l’ancien écran Médias est retiré.

---

### `media/generate-sprites-on-mannequin` · doing — reste la génération lancée depuis PCT 105, et le regard de Romain sur la page Sprites

**Pourquoi** — Les sprites dessinés cellule par cellule (à la main ou par
Claude) manquent de volume. Un modèle d'image fait bien mieux, à
condition d'imposer un format commun pour que tous les personnages
tiennent ensemble : même taille, même palette, mêmes animations.

**Périmètre** — La chaîne `scripts/pixel-art/` : un mannequin OpenPose
en trois gabarits (petit ≈ 36 px, moyen ≈ 48, grand ≈ 60) ; la
génération sur ComfyUI (tour de Romain, RTX 3080 Ti) avec SDXL, la LoRA
Pixel Art XL et ControlNet union, les trois vues (face, profil, dos)
dans une seule image, mains vides ; la normalisation (grille de 8 px,
palette maison de 32 couleurs, fond, ombre et miettes retirés) ; les
armes générées à part ; des planches de 80 × 80 px, 7 animations ×
4 directions.

**Fini quand** — Un nouveau personnage passe de sa description à sa
planche animée en une commande depuis PCT 105, et Romain valide le rendu
des quatre personnages d'essai sur la page « Fondations · Sprites ».

**Origine** — Demande de Romain, 10 et 11 octobre 2026 ; page « Fondations · Sprites » du canevas, `scripts/pixel-art/README.md`.

**État** — Fait le 11 octobre : mannequins, palette maison
(`HOUSE_32`), normalisation, découpe des vues, rig d'animation (repos,
marche, brandir, attaque, soin, touché, mort), planches de quatre
personnages d'essai (gobelin, nain, clerc, orque) publiées sur la page
« Fondations · Sprites » ; typecheck et deux tests `bun test` au vert.
Reste : `generate.ts` n'a jamais tourné depuis PCT 105, car le pare-feu
Windows de la tour n'admet que l'hôte `192.168.1.34` — il faut y ajouter
`192.168.1.192` (les générations de l'essai sont passées par un script
Python temporaire sur l'hôte). Les limites : les mouvements restent de
un à trois pixels, les bras ne bougent pas de profil, et une génération
sur trois environ est utilisable. Rien n'est commité.

### `media/draw-key-poses` · doing — reste le profil et le dos

**Pourquoi** — Le rig déplace des morceaux d'un à trois pixels : une
attaque ou un sort lisibles demandent un bras qui se lève vraiment.

**Périmètre** — Des poses clés dessinées par le modèle (élan, frappe,
incantation), d'abord de face puis de profil et de dos. Méthode : la vue
de face existante dans le panneau de gauche, le panneau de droite masqué
et complété, ControlNet sur la pose clé (`KEY_POSES` dans
`src/pose.ts`), IP-Adapter sur la référence. L'arme se pose au poignet
du squelette. Les rangées « attaque » et « soin » utilisent ces poses.

**Fini quand** — L'attaque et le soin des quatre personnages d'essai
montrent des poses dessinées, de face et de profil, sur la page Sprites.

**Origine** — Demande de Romain, 10 et 11 octobre 2026 ; page « Fondations · Sprites » du canevas, `scripts/pixel-art/README.md`.

**État** — De face, c'est fait et branché (11 octobre). Un panneau
complété par passe : à deux panneaux sur trois, les couleurs partent.
Graines retenues — élan : clerc 2, gobelin 5, nain 2, orque 6 ;
frappe : clerc 2, gobelin 4, nain 2, orque 6 ; soin : clerc 2, nain 2,
orque 1, gobelin 2. L'orque a demandé six graines avant de lever le
bras, le gobelin quatre avant de venir seul. Les chemins sont dans
`work/cast.json` (champ `keys`) ; le rig dessine la pose entière à la
place des morceaux, l'arme au poignet du squelette, et ne garde que la
forme principale du panneau (des bouts de voisins y entrent). Page
Sprites mise à jour (v100). Reste : le profil et le dos, et des
couleurs d'orque qui diffèrent un peu entre repos et poses clés.

### `media/write-pixel-art-skill` · todo

**Pourquoi** — Claude doit pouvoir enchaîner seul : décrire, générer,
choisir la bonne graine, normaliser, animer, publier.

**Périmètre** — Un skill de projet (`.claude/skills/pixel-art/`) : quand
s'en servir, les commandes, les critères pour retenir une graine (une
vraie face dans la première colonne, pas de mini-personnages collés,
pas d'arme dans les mains) et les pièges de `MEMORY.md` §4.

**Fini quand** — Dans une nouvelle session, « fais-moi un squelette
archer » donne une planche animée publiée sur la page Sprites, sans
aide.

**Dépend de** — `media/generate-sprites-on-mannequin` (le pare-feu).

**Origine** — Demande de Romain, 10 et 11 octobre 2026 ; page « Fondations · Sprites » du canevas, `scripts/pixel-art/README.md`.

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
fiches ; puis l'essai réel : ouvrir la vue joueurs de l’Atlas (l’ancienne
page `/vue-joueurs` y est entrée avec `gm/prep-on-atlas`) sur le
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

### `gm/prep-on-atlas` · doing — reste un acte préparé en entier dans l’Atlas, sur ordinateur et sur tablette (les six moments de la planche)

**Pourquoi** — Préparer et jouer doivent se ressembler : la création
finit sur la carte, et toute la préparation s'y fait.

**Périmètre** — Un écran Atlas : fil d'Ariane région › lieu › rencontre,
calques, panneau contextuel (fil de la campagne, scène, combat, lot de
Claude), atlas des cartes en bas, bascule « Vue MJ / Vue joueurs ». Il
absorbe les écrans Préparer, Médias, Cartes et Vue joueurs ; Règles,
Table et Soirée restent à part. La création de campagne s'y termine : la
première scène est posée sur la région, les lieux proposés autour en
pointillés. Ce qui n'a pas de lieu (bible, factions, chronologie) vit
dans un tiroir.

**Fini quand** — Les six moments de la planche « Atlas · préparer avec
Claude en direct » sont faisables dans l'app, sur ordinateur et sur
tablette.

**Étapes** — Coquille de l'écran (fil d'Ariane, calques, panneau
contextuel, atlas en bas), posée avec `maps/browse-atlas` → chaque
ticket du jalon y branche sa part → tiroir de ce qui n'a pas de lieu →
fin de la création de campagne sur la région → bascule nette : les
écrans Préparer, Médias, Cartes et Vue joueurs sont retirés, leurs liens
mènent à l'Atlas. La planche Atlas n'étant pas disponible, on suit les
planches Carte-MJ, Éditeur-Carte et Préparer-MJ et la direction visuelle
de `MEMORY.md`.

**Vérification** — Tests front de chaque panneau ; vérification à la
main sur ordinateur et tablette (disposition choisie en JavaScript,
comme l'écran de soirée). Essai réel : un acte préparé en entier dans
l'Atlas.

**Risques** — Un écran trop dense sur tablette : le panneau contextuel
se replie. Retirer des écrans qui marchent : la bascule n'a lieu que
quand l'Atlas fait tout ce qu'ils faisaient.

*État* — La coquille est posée avec `maps/browse-atlas`, à l'adresse
`/campagnes/:id/atlas` : fil d'Ariane, zone des calques, carte au centre,
panneau de la carte à droite (échelle, origine, validée ou brouillon,
notes du MJ, cartes où l'on entre, cartes d'à côté, lien vers l'éditeur
ou vers la copie d'une carte du monde), bande de toutes les cartes en
bas, et tiroir « Sans lieu » qui mène à la bible, aux scènes et actes, et
aux fiches de l'écran Préparer (ouvert directement sur le bon
onglet). Sur tablette, la carte se manie aux doigts et le panneau se
replie. Les calques proposés sont pour l'instant ceux de la carte
elle-même (par exemple « Secrets ») : les allumer ou les éteindre change
seulement ce que voit le MJ. Absents à dessein, faute de ce qui les
remplirait : la bascule « Vue MJ / Vue joueurs » (`maps/preview-as-player`),
le fil de la campagne (les épingles
de scène sont arrivées depuis avec `campaign/pin-scenes-on-maps` :
numérotées sur la carte, elles listent leurs scènes dans le panneau, qui
gagne l'outil « Épingler un lieu » et la liste des scènes sur aucune
carte). Ni les factions ni une chronologie n'ont d'écran de préparation
aujourd'hui : le tiroir ne les propose pas. L'Atlas se recharge seul
quand une carte change (le MJ ailleurs ou Claude), mais n'affiche pas
encore le bandeau « Claude écrit · sur … » : il viendra avec les lots de
Claude, en réutilisant le même mécanisme que l'écran Préparer. Les
anciens écrans restent en place.
Depuis `campaign/place-cast-and-clues`, la rangée « Calques de
l'histoire » allume ou éteint Histoire, Personnages, Combats et Indices.
Le panneau gagne l'outil « Poser sur la carte » (PNJ, indice, combat) et
le détail de ce qui est touché sur la carte.
Depuis `campaign/draw-story-thread`, le fil de la campagne relie les
épingles, et le panneau commence par la liste des scènes de la carte,
synchronisée avec elle et filtrable par acte.
Depuis `platform/propose-edits-by-claude`, le panneau commence par les
propositions de Claude (garder, jeter avec un mot, tout garder, et le
réglage « Claude a la main »), ce qu'elles ajouteraient est dessiné en
pointillés marqués ▶ sur la carte, et le bandeau « Claude écrit · sur … »
s'affiche au-dessus de la carte, avec son « Annuler ».

Depuis `maps/preview-as-player`, la bascule « Vue MJ / Vue joueurs » est
en haut de l’écran.
*Bascule* (11 octobre 2026) — L’Atlas est maintenant le seul écran de
préparation. Les écrans Préparer, Médias, Cartes et Vue joueurs sont
retirés, leurs adresses n’existent plus, et tout ce qu’ils faisaient
se fait dans l’Atlas :
- le **tiroir** (ce qui n’a pas de lieu) ouvre, à la place de la carte,
  Scènes et actes (graphe, fiche de scène, atelier du co-MJ), Bible,
  Fiches, Cohérence, Valider (jauge, validation, invitation), Médias
  (images et vidéos à dessiner, garder ou refaire) et Cartes de la
  campagne (carte vierge, copie d’une carte du monde, carte générée pour
  une scène, import) ; « Retour à la carte » le referme ; l’adresse garde
  l’onglet et la scène ouverte ;
- une scène touchée sur une épingle ou dans la liste ouvre sa fiche dans
  le tiroir, et « L’épingler sur la carte » ramène sur la carte ;
- la **vue joueurs** remplace l’ancien écran : la carte projetée par le
  serveur, et à côté ce que disent les écrans des joueurs ;
- l’éditeur de carte reste un outil à part, ouvert depuis l’Atlas, et il
  y ramène sur la carte éditée ;
- la page de la campagne ne propose plus que l’Atlas, la génération
  (tant que la campagne n’est pas validée), la soirée, la table et les
  règles : Règles, Table et Soirée restent à part.
*Fin de la création* — Une campagne générée s’ouvre sur l’Atlas : en
l’appliquant, sa scène d’ouverture est posée sur la région du monde (près
du lieu de la région qui porte le nom de son lieu, sinon près du départ
du groupe, sinon au milieu), dans la même ligne du journal que le
brouillon, au nom du co-MJ ; une scène déjà épinglée n’est pas déplacée.
Les autres lieux proposés autour en pointillés ne sont **pas faits** :
les propositions dessinées sur la carte sont aujourd’hui celles de
Claude (jeton), et y ajouter celles du co-MJ voudrait élargir ce
mécanisme ; à trancher. Une campagne créée sans génération n’a pas de
scène à poser.
*Reste* — Les six moments de la planche joués sur ordinateur et sur
tablette, un acte entier préparé dans l’Atlas.
**Origine** — Demande de Romain, 10 octobre 2026 ; planche « Atlas · préparer avec Claude en direct » (page « MJ · Créer, préparer »).

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

### `characters/show-generated-sprites-on-plan` · todo

**Pourquoi** — Juger les sprites générés là où ils serviront : sur la
carte, à côté des tuiles.

**Périmètre** — Dans le canevas, les composants `Plan` et `Sprite`
affichent les planches générées pour les personnages qui en ont une
(nain → Borin, clerc → Lyra, gobelin, orque → le chef), à l'échelle des
cases (52 à 58 px), avec repos, attaque et touché. Les autres gardent
leur sprite actuel.

**Fini quand** — Les planches « Atlas » et « Carte » montrent les
nouveaux sprites, et Romain dit si l'échelle et la vue de trois-quarts
tiennent avec les tuiles.

**Origine** — Demande de Romain, 10 et 11 octobre 2026 ; page « Fondations · Sprites » du canevas, `scripts/pixel-art/README.md`.

### `characters/choose-sprite-source` · done — HD-2D avec des éléments PixelLab

**Pourquoi** — L'essai généré contredit trois décisions de `MEMORY.md`
§2 : la vue de profil façon Terraria, le personnage décrit en calques et
dessiné par le serveur, et les attaques animées sur le sprite entier.
Deux sources de sprites ne peuvent pas cohabiter à l'écran.

**Options** — (a) garder les calques pour les personnages des joueurs
(le créateur) et générer PNJ et monstres, au risque de deux styles ;
(b) tout générer, et le créateur devient une description plus un choix
parmi des propositions, ce qui perd l'édition fine et le rendu serveur
unique ; (c) se servir du modèle pour concevoir les pièces des packs,
puis les reporter en calques. ; (d) **HD-2D façon Octopath Traveler** (idée de
Romain, 11 octobre) : des personnages en pixel art posés comme des
panneaux plats dans une scène 3D Three.js, la 3D apportant lumière,
ombres, flou de profondeur. Variante : sprites de 64 px sur le plateau,
portrait détaillé de 256 px en gros plan (action, dialogue, fiche, écran
TV). Remet en cause la vue de profil et le rendu serveur unique ; à
maquetter sur le canevas avant de trancher.

**Test PixelLab (11 octobre)** — Les quatre personnages d'essai en
4 directions, 64 px, mode standard (`work/pixellab/`, 4 générations ;
26 restent sur l'essai gratuit, une génération à la fois). Bien
meilleur que notre chaîne : contour net, directions cohérentes, armes
tenues. Limites : même carrure pour tous sans réglage des proportions,
arme incluse dans le dessin. Pas encore testé : proportions sur mesure,
animations, sprite de carte redessiné à partir d'un portrait de 256 px.

**Prototype HD-2D (11 octobre)** — La bagarre du quai des Corsaires
(acte 1, scène 4) montée en Three.js à partir de la carte
`quai-port-louis` : lumière des lanternes, brouillard, ombres, flou de
profondeur, une passe de combat avec les vrais jets
(`docs/design/hd2d/quai.html`, artefact « Quai de
Port-Louis »). Test PixelLab de 9 générations dedans : Gueule-Rouge et
un marin animés, une façade d'entrepôt (excellente pour 1 génération),
des lanternes, une texture de face de caisse. Romain a préféré les
caisses en boîtes 3D texturées aux caisses en image plate.

**Décision (Romain, 11 octobre)** — Option (d). La plateforme rend en
HD-2D avec Three.js, et les éléments viennent de PixelLab : ce qui a du
volume est une forme 3D texturée par des images vues de face ; les
personnages et ce qui est fin ou irrégulier sont des images plates.
`MEMORY.md` §2 est à jour. Reste à écrire les tickets qui remplacent la
vue de profil, les calques et le rendu serveur, et à choisir un
abonnement PixelLab : l'essai gratuit (17 générations restantes) ne
donne pas accès aux tuiles haut de gamme ni aux kits de bâtiment.

**Origine** — Demande de Romain, 10 et 11 octobre 2026 ; page « Fondations · Sprites » du canevas, `scripts/pixel-art/README.md`.

## Épic `hd2d`

Décision : `MEMORY.md` §2 (« the platform renders in HD-2D »).
Prototype de référence : la bagarre du quai des Corsaires,
`docs/design/hd2d/quai.html` (artefact « Quai de
Port-Louis »). Règle de forme : ce qui a du volume est une forme 3D
texturée par des images PixelLab vues de face ; les personnages et ce
qui est fin ou irrégulier sont des images plates tournées vers la
caméra.

Règle commune : **la grille porte toujours les règles**. La scène 3D
n'est qu'une projection des données que le serveur a déjà filtrées
(brouillard, pions cachés, calques du MJ) ; le serveur reste seul juge
du déplacement, de la portée et de la ligne de vue.

### `media/choose-pixellab-plan` · doing — abonnement pris ; restent le budget par monde dans `MEMORY.md` et la vérification des conditions

**Pourquoi** — L'essai gratuit (40 générations, 17 restantes) ne donne
pas accès aux tuiles haut de gamme ni aux kits de bâtiment, et une
scène complète coûte environ 120 à 145 générations.

**Périmètre** — Choisir l'abonnement ; estimer le coût d'un monde (le
port des Corsaires, le Cure-Dent du Brasier) en comptant ce qui se
réutilise d'une scène à l'autre ; vérifier dans les conditions de
PixelLab que les images générées peuvent servir dans Promptus
auto-hébergé.

**Fini quand** — L'abonnement est pris ou refusé, avec le budget par
monde noté dans `MEMORY.md`.

**Origine** — Devis du 11 octobre 2026 (test à 8 générations, puis la
texture de caisse).

*État* — Romain a pris le palier 1 (« Pixel Apprentice ») le 11 octobre
2026 : 2 000 générations par cycle, remises à zéro le 11 de chaque mois
(prochain : 11 novembre 2026). Les 17 générations d'essai restantes ne
sont pas reportées.

### `hd2d/build-pixellab-asset-pack` · todo

**Pourquoi** — La scène n'est belle qu'avec des éléments cohérents
entre eux, et le MJ ne doit pas les fabriquer à la main.

**Périmètre** — Pour un thème (`content/themes/`), la liste de ce
qu'il faut générer : une texture vue de face par matière de sol et de
mur, par face d'accessoire en volume (caisse, tonneau, coque), une
façade par type de bâtiment, une image plate par accessoire fin
(lanterne, cordage, enseigne). Génération par le MCP PixelLab, couleurs
ramenées sur la palette du thème (`reduce_colors`), validation par le
MJ comme les autres images (`media_assets`). Le coût en générations est
annoncé avant chaque lot.

**Fini quand** — Le thème `port-1718` des Corsaires et un thème du
Brasier ont tous leurs éléments, validés par Romain.

**Origine** — Prototype du quai ; test PixelLab du 11 octobre 2026
(`work/pixellab/test8/`).

### `hd2d/render-board-in-three` · todo

**Pourquoi** — C'est le cœur de la décision : le plateau devient une
scène 3D éclairée au lieu d'un dessin plat.

**Périmètre** — Une scène Three.js construite à partir de `MapData` qui
remplace le canvas de `front/src/features/map/` partout où il sert
(`BoardPanel`, `TvStage`, `BattleScreen`, `PlayersView`) : sols texturés,
murs et bâtiments en volumes, accessoires selon la règle de forme, eau,
lumières de la carte (`lights`, qui vacillent), ambiance (heure, brume),
caméra inclinée qui suit l'action, ombres, halo, flou de profondeur.
Une vue à plat de secours pour un appareil trop faible.

*Point de départ* — `front/src/features/hd2d/` (Atlas,
`campaign/watch-prep-in-hd2d`) construit déjà la scène depuis la grille
et le thème ; reste à la brancher sur le plateau, la TV et les
téléphones, avec la projection joueur, et à ajouter flou et halo.

**Fini quand** — `quai-port-louis` et une carte du Brasier sont rendues
par l'app à partir de leur grille, sans image préparée, à 60 images/s
sur un téléphone milieu de gamme et sur la TV, brouillard et pions
cachés respectés côté joueur.

**Origine** — Décision du 11 octobre 2026 ; prototype du quai.

### `hd2d/show-pixellab-characters` · todo — à spécifier pour le créateur

**Pourquoi** — Les personnages sont ce que les joueurs regardent le
plus ; ceux de PixelLab sont nettement meilleurs que nos calques.

**Périmètre** — Chaque personnage et adversaire est une image plate
PixelLab dans la scène : 4 directions, animations repos, marche,
attaque, coup reçu et chute, l'ouest obtenu par miroir de l'est.
**À spécifier :** ce que devient le créateur de personnage
(`characters/build-character-creator`), sans doute une description et
un choix parmi des propositions générées, et qui paie les générations.

**Fini quand** — Les six PJ et les adversaires d'une scène de chaque
monde témoin s'affichent et s'animent sur téléphone, écran MJ et TV.

**Origine** — Décision du 11 octobre 2026 ; Gueule-Rouge et le marin
du test PixelLab.

### `hd2d/stage-combat-actions` · todo

**Pourquoi** — Le prototype a montré que la mise en scène d'une action
(élan, coup, dégâts affichés, jet expliqué) rend les règles lisibles,
ce qui manquait à la table des Corsaires.

**Périmètre** — Les événements de combat déjà émis par le serveur
(`battleEvents.ts`) pilotent la scène : déplacement, attaque au corps à
corps, tir, soin, coup raté, chute, fuite ; dégâts et jet affichés
au-dessus des personnages avec le calcul (« d20 13 + FOR 2 = 15 contre
CA 9 ») ; la caméra cadre l'acteur et sa cible. Respecte
`prefers-reduced-motion`.

**Fini quand** — Un tour complet de la bagarre du quai, joué dans
l'app, se lit sur le téléphone d'un joueur sans explication du MJ.

**Origine** — Prototype du quai.

### `hd2d/retire-flat-renderers` · todo

**Pourquoi** — Deux rendus ne doivent pas cohabiter à l'écran.

**Périmètre** — Une fois les trois tickets précédents faits : retirer
le rendu 2D des cartes et le rendu serveur des personnages, mettre à
jour `MEMORY.md` §2 (vue de profil, personnage décrit en calques,
rendu par le serveur seul), et reprendre ou archiver les tickets qu'ils
rendent caducs : `characters/render-layered-sprite`,
`characters/walk-in-four-directions`,
`characters/show-generated-sprites-on-plan`,
`maps/render-three-quarter-tiles`, `media/draw-key-poses`.

**Fini quand** — Plus aucun écran n'utilise l'ancien rendu, et
`MEMORY.md` ne décrit plus que le nouveau.

**Origine** — Décision du 11 octobre 2026.

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
MJ. Depuis `maps/edit-maps-by-ops`, il n'y a plus de bouton
Enregistrer : chaque geste part aussitôt et s'enregistre, « Annuler mon
dernier geste » le reprend par le journal de préparation, le nom et les
notes partent quand on quitte le champ. Toute modification repasse la
carte en brouillon ; seule une carte validée
est proposée au plateau (avant les cartes du monde) et peut être montrée.
Le serveur valide chaque enregistrement et répond en français. Une carte
affichée à la table ne peut pas être supprimée. Révéler un calque ou un
objet en direct existait déjà (`maps/reveal-fog-and-hidden`). Écart : pas
encore de pinceau de brouillard. Pas encore essayé sur une vraie
préparation.

**Origine** — Planches « Cartes · trois échelles » et « Cartes · l'éditeur »

*Depuis `gm/prep-on-atlas`* — La liste des cartes de la campagne (carte
vierge, copie, génération, import) est l’onglet « Cartes de la
campagne » du tiroir de l’Atlas ; l’éditeur ramène sur la carte éditée
dans l’Atlas.

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
nœud (le MJ la choisit au plateau). Depuis la fiche de scène et, avec
`campaign/pin-scenes-on-maps`, depuis une épingle de l'Atlas
(« Générer sa carte » sur une scène qui n'en a pas), la carte générée
est rattachée à la scène, et l'Atlas l'ouvre aussitôt.

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

---

### `maps/browse-atlas` · doing — reste l'essai sur un iPad : de la région des Corsaires au quai de Port-Louis puis dans une rencontre, et retour, sans saccade

**Pourquoi** — Les cartes existent déjà à trois échelles et se relient
par leurs sorties ; il manque la façon d'y circuler.

**Périmètre** — Arbre de cartes tiré des sorties (une carte enfant
ancrée sur un hexagone ou une zone du parent), fil d'Ariane, zoom avant
qui entre dans un lieu et zoom arrière qui revient d'où l'on vient.
Rendu en basse résolution pendant le mouvement, pour les téléphones et
les tablettes. Le joueur ne reçoit que la carte où il se trouve, jamais
l'atlas entier.

**Fini quand** — De la région des Corsaires, Romain entre dans le quai
de Port-Louis puis dans une rencontre, et revient à la région, sans
saccade sur un iPad.

**Étapes** — Arbre de l'atlas calculé par le serveur (cartes du monde et
de la campagne, rattachées par les ancres des lieux et les sorties) →
route MJ de lecture de l'arbre → fil d'Ariane et zoom avant / arrière sur
la carte en hexagones et la carte en cases → rendu allégé pendant le
mouvement → côté joueur, rien de neuf : il ne reçoit que la carte où il
est.

**Vérification** — Tests de l'arbre (une carte sans parent est une
racine, pas de cycle) ; balayage des routes MJ ; tests front de la
navigation. Essai réel sur un iPad.

**Risques** — Des cycles dans les sorties : l'arbre s'appuie sur les
ancres, pas sur les sorties seules. Des cartes lourdes sur tablette :
image basse résolution pendant le geste.

*État* — Le MJ a un écran Atlas (entrée « Atlas » sur la page de la
campagne) qui range toutes les cartes du monde et de la campagne en un
arbre : la région en hexagones, les lieux posés dessus, les rencontres
dans un lieu. Une carte est rangée sous une autre quand celle-ci a une
sortie vers elle **et** que l'échelle descend (région › lieu ›
rencontre) ; une sortie entre deux cartes de même échelle, ou qui
remonte, devient un lien « À côté ». Aucune boucle n'est donc possible,
quoi que disent les sorties (règle écrite dans `docs/map-format.md` et
`MEMORY.md`). Sur la carte, les cases qui ouvrent une autre carte sont
cerclées en pointillés : les toucher y entre, comme les boutons
« Entrer » du panneau. Le fil d'Ariane nomme l'échelle de chaque niveau ;
« Ressortir » ramène là d'où l'on vient (même après un saut par la bande
du bas ou un lien « À côté »), sinon à la carte parente. Quand le MJ fait
glisser ou pince une carte en cases, elle est dessinée en demi-résolution
et ses animations s'arrêtent le temps du geste, puis redeviennent nettes
(vaut partout pour un glissé à la souris, et aux doigts là où la carte se
manie aux doigts : l'Atlas et la carte de soirée du MJ sur tablette ; le
téléphone du joueur garde son défilement natif). Côté joueur, rien de
neuf : il ne reçoit toujours que la carte posée sur la table, par la
projection (l'Atlas est une route MJ, absente des routes joueur et
jeton). Hypothèses prises : les ancres des lieux de l'histoire n'existant
pas encore, l'arbre ne lit que les sorties — une carte dessinée par le MJ
reste une racine tant qu'aucune carte n'a de sortie vers elle (elles
arrivent avec `campaign/pin-scenes-on-maps`). Dans les fichiers actuels,
le quai de Port-Louis est une carte de rencontre et ses sorties mènent à
des cartes qui n'existent pas : « entrer dans une rencontre depuis le
quai » demande d'abord d'y rattacher une carte. 🔴 La fluidité sur iPad
n'est pas mesurée. Depuis `campaign/pin-scenes-on-maps`, la région des
Corsaires ouvre une nouvelle carte de lieu, « Port-Louis, le quartier
bas », dont l'appontement ouvre le quai : région › lieu › rencontre
existe désormais (et région › abords du Toboggan › coursive pour le
Brasier). Les épingles des lieux ne rangent pas les cartes : seules les
sorties le font (décision écrite dans `docs/map-format.md`).

**Origine** — Demande de Romain, 10 octobre 2026 ; planche « Atlas · préparer avec Claude en direct » (page « MJ · Créer, préparer »).

---

### `maps/edit-maps-by-ops` · doing — reste l'essai réel : le MJ peint un mur pendant que Claude pose un PNJ sur la même carte, et les deux restent

**Pourquoi** — L'éditeur réenregistre aujourd'hui la carte entière : la
dernière écriture gagne. Avec Claude qui écrit en même temps, il
effacerait ses ajouts sans rien dire.

**Périmètre** — L'éditeur et l'API passent à des opérations ciblées
(peindre une zone, poser, déplacer, retirer) avec un numéro de version ;
une version dépassée est refusée et relue, jamais écrasée. Mêmes verrou,
révision et tout-ou-rien que les modifications d'histoire.

**Fini quand** — Le MJ peint un mur pendant que Claude pose un PNJ sur
la même carte : les deux changements restent.

**Étapes** — Opérations sur une carte dans le crate partagé (peindre,
poser, déplacer, retirer), validées comme la carte entière → route
d'opérations avec la révision attendue, sous le verrou de la campagne ;
chaque lot d'opérations entre dans le **même** journal que l'histoire
(pas de version propre à la carte) → l'éditeur envoie des opérations au
lieu de la carte entière (bascule nette) → une révision dépassée est
refusée avec ce qui a changé, l'éditeur relit et rejoue.

**Vérification** — Tests : deux lots concurrents sur des cases
différentes restent tous deux ; un lot sur une révision dépassée qui
touche la même case est refusé ; l'annulation d'un lot de carte passe par
le journal. Tests front de l'éditeur.

**Risques** — Une carte validée qu'une opération rend invalide : même
règle qu'aujourd'hui, une modification retire la validation. Des lots
très fins qui gonflent le journal : regrouper un geste en un lot.

*État* — L'éditeur de carte n'envoie plus jamais la carte entière :
chaque geste (un coup de pinceau, une porte, un objet posé, une gomme,
un réglage d'ambiance, le nom, les notes) part aussitôt comme un petit
lot d'opérations, avec le numéro de révision de la campagne lu à
l'ouverture. Le serveur rejoue le lot sur la carte telle qu'elle est,
sous le verrou de la campagne, tout ou rien, puis la vérifie comme une
carte entière. Si quelqu'un d'autre a écrit entre-temps sur d'autres
cases, les deux changements restent ; s'il a touché les mêmes cases ou
le même objet, le serveur refuse le lot avec la carte à jour et n'écrit
rien par-dessus ; l'éditeur recharge alors la carte et rejoue le geste
une fois dessus (message « la carte a changé entre-temps »). Ce geste
rejoué l'emporte donc sur ce que Claude venait de mettre sur ces cases
(une porte touchée repart de son état à jour), et la ligne de Claude
reste au journal. Ce que Claude écrit
pendant que l'éditeur est ouvert arrive sans recharger. Chaque lot est
une ligne du journal de préparation commun (auteur, cases et objets
touchés, carte d'avant) ; « Annuler mon dernier geste » passe par ce
journal, remet seulement les cases et objets de ce geste (ce que Claude
a fait ailleurs reste), peut être répété geste après geste, et n'est
refusé que si un lot plus récent a touché les mêmes cases. Créer,
importer, générer ou supprimer une carte laisse aussi sa ligne. Un
jeton personnel (Claude) peut lire les cartes de la campagne et envoyer
des lots, mais ni créer, importer, supprimer ou valider une carte.
Testé côté serveur : mur du MJ et PNJ de Claude sur la même révision →
les deux restent ; même case → refus avec la carte fraîche ; annulation
par le journal, deux fois de suite malgré un lot de Claude ailleurs, et refus
quand un lot plus récent touche la même case ; l'ancien
enregistrement de la carte entière ne répond plus.
**Hypothèses retenues** : « poser un PNJ » sur une carte = un départ
rattaché au PNJ (`entity`), seul moyen existant aujourd'hui ; les vraies
positions de PNJ, indices et rencontres rangées dans l'histoire
viendront avec `campaign/place-cast-and-clues`. Un lot de Claude
s'applique encore tout de suite (il deviendra une proposition avec
`platform/propose-edits-by-claude`), et aucun outil MCP n'écrit encore
sur les cartes (`platform/edit-maps-by-claude` les a depuis branchés :
« Redessiner une carte » et « Poser sur une carte »). Une modification
d'histoire ne rend
jamais un lot de carte périmé. Valider une carte n'entre pas dans le
journal. **Reste** : l'essai réel avec Claude, dont les outils de carte
sont maintenant branchés.

**Origine** — Demande de Romain, 10 octobre 2026 ; planche « Atlas · préparer avec Claude en direct » (page « MJ · Créer, préparer »).

---

### `maps/preview-as-player` · doing — reste, sur les deux mondes, la télé posée à côté de l’Atlas pour vérifier à l’œil que la vue joueurs est la même

**Pourquoi** — Avant de lancer, le MJ vérifie ce qui partira sur la TV
et les téléphones.

**Périmètre** — Une bascule dans l'Atlas rend la carte par la projection
serveur : brouillard, PNJ non rencontrés, indices, numéros de scène et
propositions de Claude disparaissent. Aucun filtre côté client.

**Fini quand** — Sur les deux mondes, la vue joueurs de l'Atlas est
identique, élément par élément, à ce que montre la TV pour le même état.

**Étapes** — Route MJ qui rend une carte de l'Atlas par la projection
des joueurs (la même fonction que leurs routes) → bascule « Vue MJ / Vue
joueurs » dans l'Atlas, qui affiche ce que la route rend, rien de plus →
les propositions de Claude, les épingles non révélées et les positions
de préparation n'y apparaissent jamais.

**Vérification** — Test serveur : la vue joueurs de l'Atlas égale la
réponse de la route TV pour le même état, sur les deux mondes ; le
balayage des fuites des routes joueur passe dessus. Essai réel : la TV à
côté de l'Atlas.

**Risques** — Un second filtre écrit côté client pour aller plus vite :
interdit, la projection serveur est le seul passage.

*État* — La bascule « Vue MJ / Vue joueurs » est en haut de l’Atlas.
En vue joueurs, la carte est celle que le serveur envoie aux joueurs, et
rien d’autre : brouillard, PNJ non rencontrés, indices non trouvés,
épingles des lieux pas encore visités, numéros de scène et propositions
de Claude n’y sont pas, parce que la projection ne les envoie jamais. Si
la carte est sur la table, c’est exactement ce que reçoit la télé ; sinon,
c’est ce que la télé montrerait si le MJ la montrait maintenant (le groupe
sur ses départs, ou là où il est sur la région, le brouillard levé autour
de lui), calculé sans rien écrire et sans soirée ouverte. Le texte le dit
(« Sur la table en ce moment » ou « Pas encore sur la table »). À côté de
la carte, le panneau montre ce que disent les écrans des joueurs : la
campagne, son accroche, la scène ouverte, les indices trouvés et les
personnages rencontrés (l’ancien écran « Vue joueurs » y est entré). La
vue suit la soirée en direct. Un test serveur vérifie, sur les deux
mondes, la région et un lieu, que l’aperçu d’une carte est identique à
ce que la télé reçoit une fois la carte montrée, PNJ caché puis rencontré
compris ; le balayage des fuites passe dessus. *Décision* : une carte de
la campagne encore en brouillon n’a pas de vue joueurs (la table ne
reçoit que des cartes validées) — l’Atlas le dit et renvoie vers la
validation. Claude n’a pas cette route : il a déjà « ce que voient les
joueurs depuis une case » dans `ask_map`. La télé dessine maintenant,
elle aussi, les épingles des lieux visités et les indices trouvés
qu’elle recevait sans les montrer. ⚠️ Une différence de dessin
demeure : sur une région en hexagones, la télé dessine des cases
carrées, là où l’Atlas et les téléphones dessinent des hexagones ; le
contenu reçu est le même. Reste l’essai réel, la télé à côté de l’Atlas.

**Origine** — Demande de Romain, 10 octobre 2026 ; planche « Atlas · préparer avec Claude en direct » (page « MJ · Créer, préparer »).
