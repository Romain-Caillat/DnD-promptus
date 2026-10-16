# Tickets

Le travail à faire, une page par ticket. Gabarit : [carte des docs](../README.md).
Un ticket fini quitte ce dossier pour `archive/tickets/<épic>.md`.
**Pas de mock** : un ticket est fini quand il a servi à une vraie soirée, pas quand ses tests passent.

## Vue d'ensemble

```text
 GARDÉ      moteur de règles · graphe d'histoire · Atlas + Claude (MCP)
            PixelLab · scène Three.js · cartes · temps réel · projection
 ADAPTÉ     écran joueur (PC) · personnages (PixelLab, classes) · écran MJ (panneau)
 GELÉ       co-MJ · TV · rendus 2D · téléphone d'abord
 NOUVEAU    narrateur · dialogues · exploration libre · hubs · mini-jeux
```

L'ancien backlog (environ 115 000 lignes de code) est gelé dans
`archive/tickets/backlog-avant-bible.md` ; une bonne partie reste le socle du jeu.

## Épics et leurs contraintes

| Épic | Ce que ça apporte | Contrainte commune |
|---|---|---|
| `design` | les maquettes PC et HD-2D, avant le code | HTML statique dans `docs/design/v2/`, données réelles des Corsaires |
| `engine` | les règles proches de DnD 5e | les règles sont **des données** ; le serveur seul juge |
| `campaign` | l'outil de l'Auteur | le graphe d'histoire est la source ; l'Atlas l'écran ; Claude par le MCP |
| `hd2d` | le monde en HD-2D | **la grille porte toujours les règles** ; la 3D ne fait que projeter |
| `narrator` | l'IA qui raconte | elle ne dit que des branches préparées |
| `play` | ce que fait le joueur sur PC | clavier/souris et manette |
| `session` | lobby, création, convergence, savoir privé | savoir privé par joueur |
| `gm` | le panneau MJ en retrait | overrides invisibles pour les joueurs |
| `minigames` | le catalogue de mini-jeux (jalon 2) | — |
| `platform` | déploiement, doc | — |

## Jalon 1 · Le one-shot des Corsaires

```text
 Lobby ─▶ Création ─▶ Taverne de Port-Louis (hub) ─▶ Fil ─▶ Bagarre du quai
```

**Fini quand** : une soirée est jouée pour de vrai, chaque joueur sur son PC et Romain
parmi eux ; le narrateur raconte sans que Romain intervienne, sauf par le panneau MJ ;
les joueurs disent que les règles étaient claires et que chacun a eu son moment.
Monde témoin : les Corsaires, dont l'acte 1 s'ouvre dans la taverne et contient la bagarre du quai.
Chaque phase commence par ses maquettes.

- **Maquettes et règles** : [`design/write-frame-tokens`](design/write-frame-tokens.md) · [`design/draw-combat-planes`](design/draw-combat-planes.md) · [`design/draw-journal-and-sheet`](design/draw-journal-and-sheet.md) · [`design/draw-lobby-and-creator`](design/draw-lobby-and-creator.md) · [`design/draw-gm-panel`](design/draw-gm-panel.md) · [`engine/rewrite-corsaires-rules`](engine/rewrite-corsaires-rules.md) · [`engine/fix-stats-by-class`](engine/fix-stats-by-class.md)
- **Le monde** : [`hd2d/build-level-from-key-art`](hd2d/build-level-from-key-art.md) · [`hd2d/build-pixellab-asset-pack`](hd2d/build-pixellab-asset-pack.md) · [`hd2d/render-board-in-three`](hd2d/render-board-in-three.md) · [`hd2d/show-pixellab-characters`](hd2d/show-pixellab-characters.md) · [`hd2d/walk-freely-together`](hd2d/walk-freely-together.md) · [`engine/reveal-by-passive-perception`](engine/reveal-by-passive-perception.md) · [`play/use-keyboard-and-gamepad`](play/use-keyboard-and-gamepad.md)
- **Parler** : [`campaign/model-hubs-and-threads`](campaign/model-hubs-and-threads.md) · [`campaign/write-dialogue-trees`](campaign/write-dialogue-trees.md) · [`campaign/check-stat-coverage`](campaign/check-stat-coverage.md) · [`campaign/rewrite-corsaires-act-1`](campaign/rewrite-corsaires-act-1.md) · [`narrator/voice-prepared-branches`](narrator/voice-prepared-branches.md) · [`play/talk-to-npcs`](play/talk-to-npcs.md) · [`hd2d/roll-dice-in-3d`](hd2d/roll-dice-in-3d.md) · [`session/converge-to-thread`](session/converge-to-thread.md)
- **Combattre** : [`hd2d/switch-combat-planes`](hd2d/switch-combat-planes.md) · [`hd2d/stage-combat-actions`](hd2d/stage-combat-actions.md) · [`hd2d/play-critical-banner`](hd2d/play-critical-banner.md) · [`engine/time-turns`](engine/time-turns.md) · [`engine/show-enemy-intents`](engine/show-enemy-intents.md) · [`play/fight-turn`](play/fight-turn.md)
- **Autour du jeu** : [`session/project-private-knowledge`](session/project-private-knowledge.md) · [`play/keep-journal`](play/keep-journal.md) · [`play/manage-inventory`](play/manage-inventory.md) · [`play/read-sheet`](play/read-sheet.md) · [`session/open-lobby`](session/open-lobby.md) · [`session/create-character`](session/create-character.md) · [`gm/open-override-panel`](gm/open-override-panel.md) · [`hd2d/retire-flat-renderers`](hd2d/retire-flat-renderers.md)

## Jalon 2 · La campagne courte

**Fini quand** : une campagne de 4 à 6 chapitres (les Corsaires, puis le Brasier) est jouée
sur plusieurs soirées : jalons de niveau, butin, arcs personnels, mini-jeux, un échec qui
fait avancer, le résumé « la dernière fois » au lobby ; entre deux chapitres, Romain
réécrit la suite avec Claude.

**À spécifier après la soirée du jalon 1** (pas encore de page) :

- `engine/rewrite-brasier-rules` le Brasier près de la 5e · `engine/level-by-milestone` (départ `engine/level-up`) · `engine/set-death-difficulty` (départ `engine/save-against-death`)
- `campaign/split-into-chapters` · `campaign/branch-on-defeat` · `campaign/write-personal-arcs` · `campaign/prepare-loot`
- `play/replace-fallen-character` (départ `player/face-death`) · `narrator/recap-last-chapter` (départ `session/write-recaps`)
- `gm/rewrite-between-chapters` (départ `platform/propose-edits-by-claude`)
- `minigames/build-catalogue` · `minigames/play-dice` · `minigames/pick-lock` · `minigames/haggle`

## Hors jalon

[`platform/deploy-self-hosted`](platform/deploy-self-hosted.md) · [`platform/publish-docs-site`](platform/publish-docs-site.md). Plus tard : le jeu sur téléphone,
d'autres types de mini-jeux (voir aussi les règles à définir).

## Tous les tickets

| Ticket | Titre | État |
|---|---|---|
| [`campaign/check-stat-coverage`](campaign/check-stat-coverage.md) | Vérifier une occasion par stat | à faire |
| [`campaign/model-hubs-and-threads`](campaign/model-hubs-and-threads.md) | Modéliser hubs et fils | à faire |
| [`campaign/rewrite-corsaires-act-1`](campaign/rewrite-corsaires-act-1.md) | Réécrire l'acte 1 des Corsaires en one-shot | à faire |
| [`campaign/write-dialogue-trees`](campaign/write-dialogue-trees.md) | Écrire les arbres de dialogue | à faire |
| [`design/draw-combat-planes`](design/draw-combat-planes.md) | Dessiner les trois plans du combat | à faire |
| [`design/draw-gm-panel`](design/draw-gm-panel.md) | Dessiner le panneau MJ | à faire |
| [`design/draw-journal-and-sheet`](design/draw-journal-and-sheet.md) | Dessiner journal, inventaire et fiche | à faire |
| [`design/draw-lobby-and-creator`](design/draw-lobby-and-creator.md) | Dessiner le lobby et la création | à faire |
| [`design/write-frame-tokens`](design/write-frame-tokens.md) | Porter le cadre Octopath dans le code | à faire |
| [`engine/fix-stats-by-class`](engine/fix-stats-by-class.md) | Fixer les stats par la classe | à faire |
| [`engine/reveal-by-passive-perception`](engine/reveal-by-passive-perception.md) | Révéler selon la Perception passive | à faire |
| [`engine/rewrite-corsaires-rules`](engine/rewrite-corsaires-rules.md) | Réécrire les règles des Corsaires près de la 5e | à faire |
| [`engine/show-enemy-intents`](engine/show-enemy-intents.md) | Montrer les intentions ennemies | à faire |
| [`engine/time-turns`](engine/time-turns.md) | Minuter les tours de combat | à faire |
| [`gm/open-override-panel`](gm/open-override-panel.md) | Ouvrir le panneau d'override | à faire |
| [`hd2d/build-level-from-key-art`](hd2d/build-level-from-key-art.md) | Construire un niveau depuis une illustration | à faire |
| [`hd2d/build-pixellab-asset-pack`](hd2d/build-pixellab-asset-pack.md) | Générer le pack d'éléments d'un thème | à faire |
| [`hd2d/play-critical-banner`](hd2d/play-critical-banner.md) | Jouer la banderole de critique | à faire |
| [`hd2d/render-board-in-three`](hd2d/render-board-in-three.md) | Rendre le plateau dans Three.js | à faire |
| [`hd2d/retire-flat-renderers`](hd2d/retire-flat-renderers.md) | Retirer les rendus 2D | à faire |
| [`hd2d/roll-dice-in-3d`](hd2d/roll-dice-in-3d.md) | Rouler un vrai dé en 3D | à faire |
| [`hd2d/show-pixellab-characters`](hd2d/show-pixellab-characters.md) | Afficher les personnages PixelLab | à faire |
| [`hd2d/stage-combat-actions`](hd2d/stage-combat-actions.md) | Mettre en scène les actions de combat | à faire |
| [`hd2d/switch-combat-planes`](hd2d/switch-combat-planes.md) | Passer d'un plan de combat à l'autre | à faire |
| [`hd2d/walk-freely-together`](hd2d/walk-freely-together.md) | Se promener librement ensemble | à faire |
| [`narrator/voice-prepared-branches`](narrator/voice-prepared-branches.md) | Raconter les branches préparées | à faire |
| [`platform/deploy-self-hosted`](platform/deploy-self-hosted.md) | Héberger Promptus chez Romain | en cours |
| [`platform/publish-docs-site`](platform/publish-docs-site.md) | Lire la doc depuis la plateforme | en cours |
| [`play/fight-turn`](play/fight-turn.md) | Jouer son tour de combat | à faire |
| [`play/keep-journal`](play/keep-journal.md) | Tenir son journal | à faire |
| [`play/manage-inventory`](play/manage-inventory.md) | Gérer son inventaire | à faire |
| [`play/read-sheet`](play/read-sheet.md) | Lire sa fiche | à faire |
| [`play/talk-to-npcs`](play/talk-to-npcs.md) | Parler aux PNJ | à faire |
| [`play/use-keyboard-and-gamepad`](play/use-keyboard-and-gamepad.md) | Jouer au clavier et à la manette | à faire |
| [`session/converge-to-thread`](session/converge-to-thread.md) | Converger du hub vers le fil | à faire |
| [`session/create-character`](session/create-character.md) | Créer son personnage | à spécifier |
| [`session/open-lobby`](session/open-lobby.md) | Ouvrir le lobby | à faire |
| [`session/project-private-knowledge`](session/project-private-knowledge.md) | Projeter un savoir privé par joueur | à faire |
