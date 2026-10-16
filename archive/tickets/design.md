# Archive — épic `design`

Les écrans dessinés avant le code, sur le canevas privé
https://claude.ai/artifact/AP3z1S5hbhqEiPzi5agTdy. Sources et mode
d'emploi : `docs/design/README.md` ; décisions : `MEMORY.md` §2 ;
quelle planche dessine quel écran : `docs/user-journeys.md`.

| Ticket | Ce qui a été livré |
| --- | --- |
| `design/map-user-journeys` | Parcours du MJ, du joueur et de la TV (`docs/user-journeys.md`, planche « Parcours ») ; les cinq questions ouvertes tranchées par les planches (`MEMORY.md` §1) |
| `design/build-design-system` | Gemmes, cœurs, cases, horloges, boutons-cartes, raretés, objets, dés, sprites, états, notifications ; les tokens écrits pour le code passent à `ui/write-design-tokens` |
| `design/draw-player-screens` | Soirée de Marc jouable (« Jouer »), Rejoindre (« Inviter »), créateur en couches (« Créer »), entre deux sessions, voyage, joueur sur ordinateur, mort d'un personnage, chacun avec son storyboard |
| `design/draw-gm-screens` | Préparation, soirée côté MJ, invitation, lancement avec la TV, voyage, éditeur de règles, éditeur de carte, tablette, chacun avec son storyboard |
| `design/verify-canvas-rendering` | Planches de la soirée et anciennes planches vues dans Chrome et corrigées le 3 octobre 2026. Les planches du 4 octobre n'ont été vérifiées que par leurs tests de logique : décision de Romain, le reste se vérifie sur la vraie plateforme |
| `design/scope-legacy-boards` | Abandonné : les anciennes planches gardent des classes non préfixées ; sans objet maintenant que le canevas n'évolue plus |

Clos le 4 octobre 2026 : « le reste devra se tester avec la plateforme
réelle et itéré dessus » (Romain).

## Après la bible (maquettes v2, `docs/design/v2/`)

| Ticket | Ce qui a été livré |
| --- | --- |
| `design/set-pc-game-frame` | `cadre-pc.html` : les zones A à H sur la taverne du Perroquet Noyé, clavier et manette, 1920 × 1080 et 1366 × 768, vue MJ. Validé le 12 octobre 2026 : habillage B · Octopath, groupe en colonne à gauche, ZQSD (`MEMORY.md` §0). Le report dans le code passe à `design/write-frame-tokens` |
| `design/draw-explore-screen` | `exploration.html` : planche jouable du Perroquet Noyé (déplacement libre, collisions, profondeur, autres joueurs en direct, [!] [?] et choses sans marqueur, une action à portée), la même salle vue par Yann et par Léa. Décidé le 12 octobre 2026 : noms des PNJ une fois connus (narrateur compris), découverte par la Perception annoncée une fois en privé, une seule vitesse. Reporté dans `hd2d/walk-freely-together`, `engine/reveal-by-passive-perception`, `session/project-private-knowledge` |
| `design/draw-dialogue-screen` | `dialogue.html` : planche jouable du dialogue avec Mère Goulven (portrait en trois expressions, réplique, indice de Perspicacité, Persuasion DD 10 et ses quatre issues, ardoise débloquée, Léa qui rejoint et souffle). Décidé le 16 octobre 2026 par Romain : options verrouillées cachées, dé en 3D au milieu de l'écran, on ne souffle que ce qu'on est seul à débloquer. Reporté dans `play/talk-to-npcs` et `hd2d/roll-dice-in-3d` |
