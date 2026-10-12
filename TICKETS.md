# TICKETS — Backlog actif

Tout ce qui reste à construire. Un ticket = une unité livrable qui a
un « fini quand » vérifiable.

**Ce qui appartient ici :** le travail à faire.
**Ce qui n'appartient pas ici :** les décisions durables (→ `MEMORY.md`),
le jeu lui-même (→ `docs/bible.md`), le récit de ce qui a été livré
(→ `archive/tickets/`).

## Conventions

- **Épics et tickets sont nommés, jamais numérotés.** Identifiant =
  `epic/verbe-objet`, kebab-case anglais.
- **Statut** : `todo` · `doing` · `blocked` (avec la raison).
  Un ticket fini quitte ce fichier pour `archive/tickets/<epic>.md`.
- **Origine** : la section de `docs/bible.md` qui l'exige.
- **Point de départ** : le code déjà écrit qu'il reprend, et le ticket
  gelé correspondant dans `archive/tickets/backlog-avant-bible.md`.
- Un ticket marqué **à spécifier** n'est pas implémentable en l'état :
  il faut d'abord écrire ses critères d'acceptation.
- **Pas de mock (règle 4)** : un ticket est « fini » quand il a servi à
  une vraie soirée, pas quand ses tests passent.

## Vue d'ensemble

Le 11 octobre 2026, Promptus est passé d'un outil pour le MJ à un jeu
« à la Baldur's Gate » (`docs/bible.md`). L'ancien backlog est gelé dans
`archive/tickets/backlog-avant-bible.md` : il a produit environ
115 000 lignes de code, dont une bonne partie reste le socle du jeu.

```
 GARDÉ      moteur de règles · graphe d'histoire · Atlas + Claude (MCP)
            PixelLab · scène Three.js · cartes · temps réel · projection
 ADAPTÉ     écran joueur (PC) · personnages (PixelLab, classes) · écran MJ (panneau)
 GELÉ       co-MJ · TV · rendus 2D · téléphone d'abord
 NOUVEAU    narrateur · dialogues · exploration libre · hubs · mini-jeux
```

| Épic | Ce que ça apporte |
| --- | --- |
| `design` | Les maquettes des nouveaux écrans, PC et HD-2D, avant le code |
| `engine` | Les règles proches de DnD 5e, en données : stats par classe, Perception, intentions, temps du tour |
| `campaign` | L'outil de l'Auteur : hubs, fils, dialogues, couverture des stats, préparation avec Claude |
| `hd2d` | Le monde en HD-2D : scène 3D, personnages PixelLab, déplacement libre, caméras du combat |
| `narrator` | L'IA qui raconte et fait parler les PNJ, dans le cadre préparé |
| `play` | Ce que fait le joueur sur PC : explorer, parler, combattre, son journal |
| `session` | Lobby, création du personnage, convergence, savoir privé |
| `gm` | Le panneau MJ en retrait et la réécriture entre chapitres |
| `minigames` | Le catalogue de mini-jeux (jalon 2) |
| `platform` | Déploiement (inchangé) |

## Ordre de construction

**Monde témoin : les Corsaires.** Leur acte 1 s'ouvre déjà dans une
taverne de Port-Louis et contient la bagarre du quai : c'est le one-shot
du jalon 1. Le Brasier suit au jalon 2. Les deux systèmes de règles
sont **réécrits au plus près de DnD 5e, sans sa complexité**
(`engine/rewrite-corsaires-rules`).

**Chaque phase commence par ses maquettes** (épic `design`). Le
prototype `docs/design/hd2d/quai.html` sert de point de départ
visuel. Les planches du canevas actuel (`docs/design/`) dessinent
l'ancien jeu : elles restent une référence de style, pas une
spécification.

### Jalon 1 · Le one-shot des Corsaires

```
 Lobby ─▶ Création ─▶ Taverne de Port-Louis (hub) ─▶ Fil ─▶ Bagarre du quai
```

**Fini quand** — Une soirée est jouée pour de vrai, chaque joueur sur
son PC et Romain parmi eux : lobby, création des personnages, la
taverne explorée librement (PNJ, indices, une occasion par stat), la
convergence, puis la bagarre du quai en trois plans. Le narrateur
raconte sans que Romain ait eu à intervenir, sauf par le panneau MJ.
Les joueurs disent ensuite que les règles étaient claires et que chacun
a eu son moment.

**Phase 1 · Maquettes et règles** — `design/set-pc-game-frame` ·
`design/draw-explore-screen` · `design/draw-dialogue-screen` ·
`design/draw-combat-planes` · `design/draw-journal-and-sheet` ·
`design/draw-lobby-and-creator` · `design/draw-gm-panel` ·
`engine/rewrite-corsaires-rules` · `engine/fix-stats-by-class`

**Phase 2 · Le monde** — `hd2d/build-pixellab-asset-pack` ·
`hd2d/render-board-in-three` · `hd2d/show-pixellab-characters` ·
`hd2d/walk-freely-together` · `engine/reveal-by-passive-perception` ·
`play/use-keyboard-and-gamepad`

**Phase 3 · Parler** — `campaign/model-hubs-and-threads` ·
`campaign/write-dialogue-trees` · `campaign/check-stat-coverage` ·
`campaign/rewrite-corsaires-act-1` · `narrator/voice-prepared-branches` ·
`play/talk-to-npcs` · `session/converge-to-thread`

**Phase 4 · Combattre** — `hd2d/switch-combat-planes` ·
`hd2d/stage-combat-actions` · `hd2d/play-critical-banner` ·
`engine/time-turns` · `engine/show-enemy-intents` · `play/fight-turn`

**Phase 5 · Autour du jeu** — `session/project-private-knowledge` ·
`play/keep-journal` · `play/manage-inventory` · `play/read-sheet` ·
`session/open-lobby` · `session/create-character` ·
`gm/open-override-panel` · `hd2d/retire-flat-renderers`

### Jalon 2 · La campagne courte

**Fini quand** — Une campagne de 4 à 6 chapitres (les Corsaires, puis
le Brasier) est jouée sur plusieurs soirées : jalons de niveau, butin,
arcs personnels, mini-jeux, un échec qui fait avancer, et le résumé
« la dernière fois » au lobby. Entre deux chapitres, Romain réécrit la
suite avec Claude.

`engine/rewrite-brasier-rules` · `engine/level-by-milestone` ·
`engine/set-death-difficulty` · `campaign/split-into-chapters` ·
`campaign/branch-on-defeat` · `campaign/write-personal-arcs` ·
`campaign/prepare-loot` · `play/replace-fallen-character` ·
`narrator/recap-last-chapter` · `gm/rewrite-between-chapters` ·
`minigames/build-catalogue` · `minigames/play-dice` ·
`minigames/pick-lock` · `minigames/haggle`

### Plus tard, sans jalon

L'ambiance sonore (la musique YouTube existe déjà) · le jeu sur
téléphone · d'autres types de mini-jeux.

---

## Épic `design`

Les maquettes viennent avant le code de chaque phase. Une maquette est
un fichier HTML statique dans `docs/design/v2/`, construit à partir de
`docs/design/hd2d/quai.html`, avec des données des Corsaires.

### `design/set-pc-game-frame` · todo

**Pourquoi** — Tous les écrans partagent un cadre : le monde HD-2D en
plein écran et une interface par-dessus. Il faut le fixer avant de
dessiner les écrans.

**Périmètre** — Résolutions cibles (1920×1080, 1366×768), zones fixes
de l'interface (barre du haut, PV en bas à gauche, invite d'action,
fenêtres), typographie lisible sur le décor, focus visible pour la
manette, raccourcis clavier affichés. Reprendre ou remplacer les tokens
existants (`ui/write-design-tokens`) pour un jeu PC.

**Fini quand** — Romain a validé une page de référence du cadre.

**Origine** — Bible §1, §10. *Point de départ* — `front/src/features/theme`, `docs/design/hd2d/quai.html`.

### `design/draw-explore-screen` · todo

**Périmètre** — La taverne de Port-Louis : joueurs visibles,
indicateurs `[!]` et `[?]`, indicateur caché révélé par la Perception
(la même scène vue par deux joueurs), invite d'interaction.

**Fini quand** — Maquette validée par Romain. **Origine** — Bible §6.

### `design/draw-dialogue-screen` · todo

**Périmètre** — Un dialogue avec un PNJ des Corsaires : portrait et
expressions, réplique, indice de Perspicacité, choix avec compétence et
DD, option débloquée, d20 qui roule, joueur qui rejoint et souffle un
choix.

**Fini quand** — Maquette validée par Romain. **Origine** — Bible §7.

### `design/draw-combat-planes` · todo

**Périmètre** — La bagarre du quai en trois plans : vue tactique
(frise d'initiative, cases, portées, ressources, aperçu, minuteur de
60 s, intentions ennemies), le coup en HD-2D, la banderole de critique.
Transitions entre les plans.

**Fini quand** — Maquette validée par Romain, transitions comprises.
**Origine** — Bible §8.

### `design/draw-journal-and-sheet` · todo

**Périmètre** — Les trois onglets Journal, Inventaire et Fiche : notes
du joueur, Partager, onglet Groupe, comparaison ▲▼, Donner à un joueur
proche, chaque stat avec son effet en combat et hors combat.

**Fini quand** — Maquette validée par Romain. **Origine** — Bible §10.

### `design/draw-lobby-and-creator` · todo

**Périmètre** — Le lobby (joueurs, prêt, classes prises, lien
d'invitation, résumé, Lancer) et la création (race → classe →
historique → apparence → nom, prétirés).

**Fini quand** — Maquette validée par Romain. **Origine** — Bible §5, §10.

### `design/draw-gm-panel` · todo

**Périmètre** — Le panneau MJ par-dessus l'écran de jeu : joueurs et
activité, alertes, actions d'override, journaux.

**Fini quand** — Maquette validée par Romain. **Origine** — Bible §11.

---

## Épic `engine`

Les règles restent **des données** (`MEMORY.md`, « Rules are data »).
Le serveur reste seul juge de chaque règle.

### `engine/rewrite-corsaires-rules` · todo

**Pourquoi** — L'esprit DnD : les joueurs doivent retrouver ce qu'ils
connaissent, sans la complexité de la 5e.

**Périmètre** — Réécrire `content/rules/corsaires/` à partir du
préréglage SRD (`content/rules/srd/`) : six caractéristiques et leurs
modificateurs 5e, compétences, bonus de maîtrise, CA, avantage et
désavantage, Action / Action bonus / Réaction / Déplacement, jets
contre la mort. On garde les classes et l'ambiance des Corsaires, avec
peu de sorts et d'aptitudes par classe jusqu'au niveau 5. Les défauts
connus de la version 1 disparaissent. Ce qui n'est pas modélisé est
écrit (`NOT MODELLED`).

**Fini quand** — `engine/lint-rule-system` ne signale plus rien de
bloquant, la simulation de la bagarre du quai est équilibrée, et Romain
a relu le fichier.

**Origine** — Bible §5, décision du 12 octobre 2026. *Point de départ* —
`engine/add-srd-preset`, `engine/simulate-fights`.

### `engine/fix-stats-by-class` · todo

**Périmètre** — Chaque classe déclare la répartition du tableau
standard (15, 14, 13, 12, 10, 8). La création n'offre plus de choix de
stats. L'Auteur peut limiter les classes d'une campagne.

**Fini quand** — Un personnage de chaque classe des Corsaires est créé
sans choisir ses stats. **Origine** — Bible §5.

### `engine/reveal-by-passive-perception` · todo

**Périmètre** — Un élément préparé (PNJ, objet, passage, indicateur)
peut porter un seuil de Perception passive. Le serveur ne l'envoie qu'aux
joueurs qui l'atteignent, par la projection. Même mécanisme pour un
indice de Perspicacité en dialogue.

**Fini quand** — Deux joueurs aux Perceptions différentes ne voient pas
la même taverne, et un test de fuite le prouve.

**Origine** — Bible §6. *Point de départ* — `maps/reveal-fog-and-hidden`, la projection joueur.

### `engine/time-turns` · todo

**Périmètre** — 60 secondes par tour de combat, données du système de
règles. Une fois le temps écoulé, le serveur fait jouer l'action
Esquiver. Le MJ peut mettre en pause.

**Fini quand** — Un tour expiré passe en Esquiver chez tous les joueurs. **Origine** — Bible §8.

### `engine/show-enemy-intents` · todo

**Périmètre** — Au début du tour d'un ennemi, le serveur choisit sa
cible et son action à l'avance. Il ne les envoie qu'aux joueurs dont la
Perspicacité atteint le seuil.

**Fini quand** — Un joueur perspicace voit la cible du gobelin avant son
tour, un autre non. **Origine** — Bible §8. *Point de départ* — `copilot/propose-adversary-turns` (gelé).

---

## Épic `campaign`

L'outil de l'Auteur. Le graphe d'histoire reste la source
(`MEMORY.md`, « Story is a graph »), l'Atlas reste l'écran de
préparation, et Claude y travaille par le MCP.

### `campaign/model-hubs-and-threads` · todo

**Périmètre** — Une scène est un **hub** (exploration libre, chacun de
son côté) ou un **fil** (groupe réuni). Un hub déclare sa
convergence : un événement préparé, tout le monde prêt, ou un joueur
qui déclenche le fil. Champs facultatifs : un ancien fichier s'importe
toujours.

**Fini quand** — La taverne de Port-Louis est un hub, avec sa
convergence vers le fil, et le validateur la vérifie.

**Origine** — Bible §3. *Point de départ* — `campaign/model-story-graph`, `campaign/edit-scenes-in-one-place`.

### `campaign/write-dialogue-trees` · todo

**Périmètre** — Le format d'un dialogue de PNJ : nœuds, répliques
(une intention que le narrateur dira à sa façon), choix, jets
(compétence et DD), options débloquées par une stat ou un savoir, et
leurs conséquences (indice, quête, objet, réputation, sortie). Validé
comme le reste du graphe. Claude l'écrit par le MCP.

**Fini quand** — Les PNJ de la taverne ont leurs dialogues, écrits avec
Claude, et le validateur les accepte. **Origine** — Bible §2, §7.

### `campaign/check-stat-coverage` · todo

**Périmètre** — Le validateur signale un hub qui n'offre aucune
occasion à l'une des six stats. Ce signal entre dans la jauge
« Jouable ».

**Fini quand** — La taverne passe au vert sur les six stats. **Origine** — Bible §5. *Point de départ* — `campaign/check-act-readiness`.

### `campaign/rewrite-corsaires-act-1` · todo

**Périmètre** — Réécrire l'acte 1 des Corsaires en one-shot au
nouveau format : la taverne (hub, PNJ, dialogues, indices, une
occasion par stat), la convergence, le fil, la bagarre du quai, la
défaite comme branche. Les éléments PixelLab sont générés et placés.

**Fini quand** — L'acte est « Jouable » dans l'Atlas et Romain l'a relu.

**Origine** — Bible §3, §4. *Point de départ* — `campaign/rewrite-two-worlds`, `campaign/prepare-with-claude`, `campaign/import-pixellab-assets`.

---

## Épic `hd2d`

Règle commune : **la grille porte toujours les règles**. La scène 3D
n'est qu'une projection des données que le serveur a déjà filtrées.

### `hd2d/build-pixellab-asset-pack` · todo

**Périmètre** — Repris tel quel du backlog gelé : la liste des éléments
d'un thème (textures, façades, accessoires), générés par le MCP
PixelLab, couleurs ramenées sur la palette, validés par le MJ, coût
annoncé avant chaque lot.

**Fini quand** — Le thème `port-1718` a tous ses éléments, validés par Romain.

### `hd2d/render-board-in-three` · todo

**Périmètre** — Repris du backlog gelé, pour le **PC** : la scène
Three.js construite depuis `MapData` remplace le canvas partout. Sols,
murs, bâtiments, accessoires, eau, lumières, brume, ombres, halo, flou
de profondeur. Caméra basse en exploration.

**Fini quand** — La taverne et le quai de Port-Louis sont rendus depuis
leur grille, à 60 images/s sur un PC de joueur, brouillard et éléments
cachés respectés.

**Origine** — Bible §12. *Point de départ* — `front/src/features/hd2d/`, `campaign/watch-prep-in-hd2d`.

### `hd2d/show-pixellab-characters` · todo

**Périmètre** — Repris du backlog gelé : chaque personnage est une
image plate PixelLab (4 directions, repos, marche, attaque, coup reçu,
chute). Le créateur est spécifié par `session/create-character`.

**Fini quand** — Les PJ et les PNJ de la taverne s'affichent et s'animent.

### `hd2d/walk-freely-together` · todo

**Pourquoi** — En exploration, on se promène sans cases, et on voit les
autres joueurs.

**Périmètre** — Déplacement continu au clavier, au clic ou à la
manette, collisions déduites des murs de la grille. Positions envoyées
au serveur, qui les valide et les diffuse aux joueurs du même lieu.
Interaction à portée (une touche, une seule action proposée). La grille
réapparaît quand un combat commence sur place.

**Fini quand** — Quatre joueurs se promènent dans la taverne en même
temps, sans traverser de mur, et chacun voit les autres bouger.

**Origine** — Bible §6. *Point de départ* — `characters/walk-in-four-directions`, `session/stream-live-changes`.

### `hd2d/switch-combat-planes` · todo

**Périmètre** — Les trois plans du combat dans la même scène 3D : la
caméra monte pour la vue tactique (grille, portées), plonge pour le
coup, puis revient. Les animations des ennemis sont accélérées ou
désactivables.

**Fini quand** — Un tour de la bagarre du quai passe par les plans sans
coupure ni chargement. **Origine** — Bible §8.

### `hd2d/stage-combat-actions` · todo

**Périmètre** — Repris du backlog gelé : les événements de combat du
serveur pilotent la scène (déplacement, coups, tirs, soins, chutes), et
le jet est expliqué au-dessus des personnages.

**Fini quand** — Un tour complet de la bagarre du quai se lit sans explication du MJ.

### `hd2d/play-critical-banner` · todo

**Périmètre** — Sur un critique ou un sort marqué « ultime », une
banderole : le portrait « en action » du personnage (image IA générée à
la création), qui glisse et zoome, avec une réplique. Elle est toujours
jouée.

**Fini quand** — Un critique joué à la soirée déclenche la banderole chez tous. **Origine** — Bible §8.

### `hd2d/retire-flat-renderers` · todo

**Périmètre** — Repris du backlog gelé : retirer les rendus 2D et les
calques de personnages, puis mettre à jour `MEMORY.md` §2.

**Fini quand** — Plus aucun écran n'utilise l'ancien rendu.

---

## Épic `narrator`

### `narrator/voice-prepared-branches` · todo

**Pourquoi** — C'est le jeu qui raconte (règle 1 : structure figée,
voix vivante).

**Périmètre** — À chaque nœud (description d'un lieu, réplique d'un PNJ,
résultat d'un jet), l'IA reçoit l'intention préparée, le contexte
court (lieu, PNJ, ce que *ce* joueur sait) et rend **du texte
seulement**. Les choix proposés, les jets et les conséquences viennent
du graphe, jamais de l'IA. Le texte est filtré (pas de noms inconnus du
joueur, pas de secret qu'il ne connaît pas). Si l'IA échoue ou tarde,
le texte préparé s'affiche tel quel. Chaque appel est compté.

**Fini quand** — Une soirée entière se joue sans que l'IA n'ait inventé
de choix, de conséquence ou de secret, et un test de fuite le prouve.

**Origine** — Bible §2, §7. *Point de départ* — `copilot/draft-narration` (gelé), `ai/route-llm-provider`, `ai/count-ai-calls`.

---

## Épic `play`

### `play/use-keyboard-and-gamepad` · todo

**Périmètre** — Clavier/souris et manette sur tous les écrans de jeu :
déplacement, interaction, menus, choix de dialogue, combat. Les
raccourcis et boutons affichés suivent l'appareil utilisé.

**Fini quand** — Une soirée se joue entièrement à la manette. **Origine** — Bible §6.

### `play/talk-to-npcs` · todo

**Périmètre** — L'écran de dialogue : portrait, texte du narrateur,
choix, jets avec le d20 qui roule, options débloquées, indice de
Perspicacité. Un autre joueur peut rejoindre : celui qui a lancé la
conversation choisit, celui qui a rejoint voit tout et peut souffler un
choix.

**Fini quand** — À la soirée, deux joueurs mènent une conversation
ensemble. **Origine** — Bible §7. *Point de départ* — `player/play-scene` (gelé), `ui/roll-faceted-dice`.

### `play/fight-turn` · todo

**Périmètre** — Le tour du joueur en vue tactique : cases accessibles,
portées, ressources, aperçu (% de toucher et dégâts), annuler avant de
valider, minuteur. Initiative en frise.

**Fini quand** — Chaque joueur joue ses tours de la bagarre du quai
sans aide. **Origine** — Bible §8. *Point de départ* — `player/fight-turn` (gelé), `engine/run-combat`.

### `play/keep-journal` · todo

**Périmètre** — Le journal privé, rempli par le jeu (quêtes,
découvertes, personnes), avec les notes du joueur, le bouton Partager et
l'onglet Groupe.

**Fini quand** — À la soirée, un indice partagé apparaît dans l'onglet
Groupe des autres, et un indice gardé n'apparaît nulle part ailleurs.

**Origine** — Bible §3, §10. *Point de départ* — `player/read-sheet-and-journal` (gelé).

### `play/manage-inventory` · todo

**Périmètre** — La grille d'objets, la comparaison ▲▼, Équiper / Donner
(à un joueur proche seulement) / Jeter.

**Fini quand** — Un objet trouvé dans la taverne est donné à un autre
joueur à côté. **Origine** — Bible §5, §10.

### `play/read-sheet` · todo

**Périmètre** — La fiche : chaque stat montre son effet en combat et
hors combat (règle 3), à partir du système de règles.

**Fini quand** — Un joueur qui découvre DnD comprend sa fiche sans aide. **Origine** — Bible §10. *Point de départ* — `player/read-the-rules` (gelé).

---

## Épic `session`

### `session/project-private-knowledge` · todo

**Pourquoi** — Le savoir est privé : ce qu'un joueur découvre, les
autres ne le savent pas tant qu'il ne l'a pas partagé.

**Périmètre** — La projection joueur devient **par joueur** : indices,
dialogues, objets et Perception filtrent ce que chacun reçoit. Le MJ
voit tout. Tests de fuite entre deux joueurs.

**Fini quand** — Aucun test de fuite ne montre un savoir d'un joueur
chez un autre. **Origine** — Bible §3. *Point de départ* — `session/project-player-view`, `session/track-table-knowledge`.

### `session/converge-to-thread` · todo

**Périmètre** — Le passage du hub au fil : l'événement préparé, tout le
monde prêt ou un joueur qui déclenche. Les autres sont appelés, puis
tout le monde est réuni dans la scène suivante.

**Fini quand** — À la soirée, la taverne converge vers le fil sans
intervention du MJ. **Origine** — Bible §3.

### `session/open-lobby` · todo

**Périmètre** — Le lien d'invitation, les joueurs connectés et prêts,
les classes prises (une classe par joueur), le résumé, et Lancer
(réservé au MJ).

**Fini quand** — La soirée démarre depuis le lobby. **Origine** — Bible §10. *Point de départ* — `session/open-lobby`, `session/invite-and-join` (gelés).

### `session/create-character` · todo

**Périmètre** — Race → classe (une par joueur, stats fixées) →
historique → apparence (une description, PixelLab pour le sprite et
l'IA pour le portrait et le portrait « en action ») → nom. Prétirés en
option. **À spécifier :** qui paie les générations, combien de
propositions on montre.

**Fini quand** — Chaque joueur de la soirée a créé son personnage, avec
son sprite et son portrait. **Origine** — Bible §5. *Point de départ* — `characters/build-character-creator`, `hd2d/show-pixellab-characters` (gelés).

---

## Épic `gm`

### `gm/open-override-panel` · todo

**Périmètre** — Le panneau MJ par-dessus l'écran de joueur du MJ
(`Tab`) : joueurs et activité, alertes (bloqué, conversation en boucle,
temps qui dérape), pause globale, forcer/annuler un jet ou une action,
déplacer un perso, déclencher la convergence. Les overrides sont
invisibles pour les joueurs. Les routes restent derrière la garde MJ.

**Fini quand** — À la soirée, Romain débloque une situation sans que
les joueurs le voient. **Origine** — Bible §11. *Point de départ* — `gm/run-live-screen`, `gm/adjust-sheets-fast` (gelés).

---

## Jalon 2 — tickets à détailler

Ils seront spécifiés après la soirée du jalon 1.

- `engine/rewrite-brasier-rules` — le Brasier au plus près de DnD 5e, comme les Corsaires.
- `engine/level-by-milestone` — montée de niveau aux jalons, pour tout le groupe. *Point de départ* — `engine/level-up`.
- `engine/set-death-difficulty` — l'Auteur règle la mort (jets contre la mort, résurrection). *Point de départ* — `engine/save-against-death`.
- `campaign/split-into-chapters` — un chapitre = une soirée, avec un début et une fin nets.
- `campaign/branch-on-defeat` — la défaite du groupe comme branche préparée.
- `campaign/write-personal-arcs` — une quête par historique.
- `campaign/prepare-loot` — objets uniques + tables aléatoires.
- `play/replace-fallen-character` — jouer un compagnon, ou un nouveau perso au prochain hub. *Point de départ* — `player/face-death`.
- `narrator/recap-last-chapter` — « la dernière fois », à partir des journaux. *Point de départ* — `session/write-recaps`.
- `gm/rewrite-between-chapters` — réécrire la suite avec Claude par le MCP. *Point de départ* — `platform/propose-edits-by-claude`.
- `minigames/build-catalogue` — le cadre commun d'un mini-jeu habillé par l'Auteur.
- `minigames/play-dice` · `minigames/pick-lock` · `minigames/haggle` — les trois premiers types.

---

## Épic `platform`

### `platform/deploy-self-hosted` · doing — reste le choix de l'hôte et la mise en ligne

Repris tel quel du backlog gelé (`archive/tickets/backlog-avant-bible.md`).
