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
