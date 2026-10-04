# Leçons des deux parties — et la fabrique qui en sort

Romain a mené une partie et en a préparé une autre à la main, avant
Promptus. Leurs dossiers sont dans `dnd-save/` (textes versionnés,
images à côté). Ce document dit ce qui a coincé, pourquoi, et la
manière de travailler que Promptus doit rendre systématique pour que ça
ne se reproduise pas. Les décisions durables sont résumées dans
`MEMORY.md` §6 ; le travail à faire est dans `TICKETS.md`.

## 1. Ce qui s'est passé

**Corsaires de la Couronne** — one-shot pirate, 1718, six joueurs à
distance. L'acte 1 a été joué le 16 mai 2026. Ressenti du MJ : difficile
de tenir l'histoire dans la continuité, de divertir six joueurs et
d'improviser ; deux ou trois choses ratées. Ressenti des joueurs : des
règles pas assez claires, et pas appliquées pareil d'un bout à l'autre
de l'acte. Pour l'acte 2, il manquait aux joueurs des informations
nécessaires. Les actes 2 à 4 n'ont jamais été écrits.

**Le Brasier** — campagne spatiale préparée le 7 juin 2026. Le monde
et l'histoire sont solides (quatre races, quatre composants, la
réputation qui se paie, une IA de bord). Mais le combat vaisseau et
équipage est resté brouillon, et l'acte 1 n'était pas assez travaillé
pour être joué.

Romain ne tient pas à ses règles telles quelles : elles peuvent
évoluer. Le monde de la prochaine partie n'est pas choisi.

## 2. Le diagnostic, cause par cause

Les symptômes ont des causes visibles dans les dossiers. Chacune est
une chose qu'un outil peut vérifier ou porter à la place du MJ.

### Des règles qui se contredisent

Les Corsaires ont **deux modèles de dégâts** : les attaques des joueurs
font des dégâts fixes (Estocade : 3), mais les PNJ et la boutique
lancent des dés (Gueule-Rouge : sabre 1d6 + 2 ; pistolet du marché
noir : 1d8, alors que le « Tir de silex » du Canonnier fait 4). Un
joueur qui achète un pistolet ne sait plus lequel s'applique.

D'autres trous du même genre :

- **Des termes non définis.** « Précision » n'est jamais relié au jet
  d'attaque ; « CD 1 tour » peut se lire « une fois tous les deux
  tours » ou « pas deux fois de suite ».
- **Des références qui ne disent pas laquelle.** « d20 + modificateur
  de votre stat principale », alors que chaque classe en a deux.
- **Des valeurs qui sortent des tables.** Les gardes royaux ont CA 14
  quand la table de référence donne 12 au soldat entraîné ; le « Kit de
  soins » donne « +1 aux jets de soin », mais les soins ne se lancent pas.
- **Une économie qui s'emballe.** +1 XP par touche, avec deux actions
  par tour et des attaques sans recharge : un combat donne plusieurs XP
  par joueur et par tour, et l'amélioration de stat en plein jeu change
  la CA au milieu d'un combat.
- **Un renvoi vers un document qui n'existe pas.** Le combat de
  vaisseau du Brasier bascule « en combat au sol (voir `Combat_Sol.md`) »,
  qui n'a jamais été écrit ; la règle « 1 seule attaque par tour » du
  vaisseau contredit les deux attaques par tour au sol.

Personne n'a tort : on écrit des règles dans quatre fichiers, on les
recopie dans un prompt, et elles divergent. **La cause est que les
règles existent en plusieurs exemplaires et qu'aucun ne se vérifie.**

### L'information critique cachée dans une scène facultative

La scène 3 de l'acte 1 propose trois lieux facultatifs, « dans l'ordre
qu'on veut, s'il reste du temps ». Or c'est là que se trouvent la route
du Greyhound et la taille de son escorte (chez Morel, payant), la
réparation du bateau (chez Thomas) et le fragment de carte (sur un
marin) — tout ce dont l'acte 2 a besoin. Les notes de partie le
confirment : « ils ont pas réparé le bateau ».

C'est exactement ce que la règle des trois indices (`MEMORY.md` §1)
interdit : une information nécessaire doit être atteignable par au
moins trois chemins. **Rien ne le vérifiait.**

### Six joueurs, une seule tête

La préparation ne dit jamais **qui** joue quand : aucune accroche par
joueur, aucune scène pensée pour un personnage en particulier. Le
combat d'apprentissage oppose six joueurs à six adversaires : douze
combattants dans l'ordre du tour, douze actions de joueurs par tour à
arbitrer, les PV notés à la main (« PV : 10 - 4 -6 »).
Pendant que le MJ calcule, cinq joueurs attendent.

### L'improvisation sans filet

Les fiches de PNJ sont bonnes (ce qu'il veut, ce qu'il cache), mais
quand les joueurs sortent du prévu, le MJ doit à la fois inventer,
rester cohérent avec ce qui a déjà été dit, et se souvenir de la
difficulté donnée la dernière fois pour la même action. Le prompt de
contexte pour un LLM existait (`prompt_contexte_dnd.md`), mais à côté
de la partie, sans mémoire de ce qui s'y passait.

### Un acte écrit trop tard, un système jamais essayé

Le Brasier a un monde complet et un acte 1 réduit à une ligne, avec le
premier contact « à trancher ». Son combat de vaisseau est écrit
« v1 à playtester » : six joueurs sur un seul vaisseau, une seule
initiative partagée, des postes, de l'énergie, des arcs, des avaries —
sans un seul combat d'essai pour voir si le tour tient en temps réel.
**On découvre qu'un système ne marche pas devant la table.**

## 3. La fabrique

Quatre chaînes, chacune avec son standard, ses contrôles et l'outil
qui les porte. Un contrôle ne bloque jamais le MJ : il signale, le MJ
décide (règle de design 1).

### Chaîne 1 · Les règles

**Standard.** Les règles n'existent qu'en un exemplaire : le système de
règles de la campagne, en données. Tout le reste en est tiré — la page
de règles des joueurs, les cartes d'action, les fiches, le prompt du
co-MJ, le calcul du serveur. Un seul modèle de dégâts pour les joueurs,
les PNJ et les objets.

**Contrôles.**
- *Cohérence* : chaque terme employé est défini, chaque renvoi existe,
  un seul modèle de dégâts, les valeurs des PNJ restent dans les tables
  de référence ou sont marquées comme exceptions.
- *Équilibre* : total des caractéristiques par classe, dégâts par tour
  et par classe, XP gagnée par session et niveau atteint en fin de
  campagne.
- *Rodage* : un combat simulé par rencontre prévue, avec les vraies
  fiches ; un tour de vaisseau simulé avant d'en jouer un.

**Évolution.** Les règles peuvent changer entre deux sessions. Chaque
version est verrouillée, et les joueurs voient « ce qui change » avant
la session suivante, jamais en pleine partie.

**Tickets.** `engine/model-rule-system`, `engine/lint-rule-system`,
`engine/simulate-fights`, `player/read-the-rules`,
`campaign/edit-rule-system`.

### Chaîne 2 · La campagne

**Standard.** Le format de préparation de Romain, complété de ce qui a
manqué :

| Une scène porte | Venu des Corsaires | Ajouté par les leçons |
| --- | --- | --- |
| Lieu, ambiance, musiques par humeur | ✓ | |
| Déroulement, accroche, jets prévus (stat, difficulté, conséquence d'un 1 et d'un 20) | ✓ | |
| PNJ (ce qu'il veut, ce qu'il cache), adversaires avec tactique | ✓ | |
| Butin, XP, transition | ✓ | |
| Ce que les joueurs doivent savoir en entrant, ce qu'ils apprennent ici | | ✓ |
| Pour chaque joueur, une raison d'agir dans la scène | | ✓ |
| Si les joueurs ne viennent pas ici : où l'information critique se trouve aussi | | ✓ |

**Contrôles.** Un acte est **prêt** quand chaque scène a ses champs,
chaque information nécessaire a au moins trois chemins, chaque joueur a
au moins une accroche dans l'acte, chaque rencontre a ses adversaires
chiffrés et sa tactique, et chaque combat prévu a été simulé. Le MJ voit
une jauge par acte, pas une liste de reproches.

**Tickets.** `campaign/model-story-graph`, `campaign/check-player-knowledge`,
`campaign/check-act-readiness`, `ai/generate-campaign`,
`media/generate-images-and-video`, `media/play-youtube-music`.

### Chaîne 3 · La soirée

**Standard.** Le MJ raconte et décide ; Promptus tient le reste.
- *La mémoire* : ce que la table sait, les promesses, les dettes, et les
  décisions de règle prises en jeu, reproposées quand la situation
  revient.
- *Le compte* : PV, XP, recharges, or, ordre du tour, tenus par le
  serveur, jamais de tête.
- *La table* : qui n'a pas eu la main depuis longtemps, quelle accroche
  personnelle n'a pas encore servi.
- *Le filet* : le co-MJ improvise un PNJ ou une conséquence à partir de
  ce que la table sait et de la fiche du PNJ ; le MJ valide.

**Tickets.** `session/track-table-knowledge`, `gm/balance-spotlight`,
`gm/adjust-sheets-fast`, `copilot/draft-narration`,
`copilot/propose-adversary-turns`, `engine/run-combat`.

### Chaîne 4 · Le retour

C'est ce qui manquait le plus : les leçons des Corsaires sont arrivées
par des reproches oraux, après coup. Elles doivent arriver à chaque
fois, par écrit, et nourrir les trois autres chaînes.

**Standard.** À la fin de chaque session, chaque joueur répond en
trente secondes sur son téléphone : *Les règles étaient-elles claires ?
As-tu eu un moment à toi ? Sais-tu ce que ton personnage veut faire la
prochaine fois ?* Le MJ voit les réponses à côté de ce que Promptus a
mesuré (temps de parole par joueur, jets contestés, informations
manquantes pour la suite), et décide ce qu'il change : une règle (chaîne
1), une scène (chaîne 2), sa manière de mener (chaîne 3).

**Tickets.** `session/collect-player-feedback`, `session/end-session`,
`session/write-recaps`.

## 4. Les deux parties comme corpus de référence

Les deux dossiers deviennent des jeux de données permanents, pas des
souvenirs :

- **L'acte 1 des Corsaires** est réécrit dans le format de Promptus
  (`campaign/rewrite-two-worlds`), visuels refaits en pixel art ; la
  version jouée reste dans `dnd-save/` comme cas de test. Les contrôles doivent y retrouver
  ce que la partie a révélé : l'information critique cachée dans la
  scène facultative, les deux modèles de dégâts, les termes non définis.
  Si un contrôle ne voit pas ces défauts, il ne verra pas ceux de la
  prochaine campagne.
- **Le Brasier** sert de test au moteur : ses classes, son vaisseau
  et ses factions doivent pouvoir s'écrire dans le système de règles ; sa
  jauge de préparation doit dire que l'acte 1 n'est pas prêt.
- **Les situations réelles de l'acte 1** (refuser l'offre de
  Vaubernier, l'incident de Jacquot, le combat du quai à six contre six)
  deviennent les cas d'évaluation du co-MJ et de la génération
  (`ai/evaluate-on-real-campaigns`) : chaque changement de prompt ou de
  modèle est rejoué dessus.

## 5. Ce qui change dans le jalon 1

Le jalon 1 est jugé sur les quatre douleurs des Corsaires, mesurées par
la chaîne du retour : après une soirée à six joueurs, les réponses des
joueurs disent que les règles étaient claires, que chacun a eu son
moment, et que chacun sait quoi faire ensuite. Les tickets ajoutés par
ce document — `engine/lint-rule-system` et
`session/collect-player-feedback` au jalon 1,
`campaign/check-act-readiness` et `ai/evaluate-on-real-campaigns` au
jalon 2 — sont décrits dans `TICKETS.md`.
