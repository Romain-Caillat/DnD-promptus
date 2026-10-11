# Bible du jeu — Promptus

> Issue du brainstorming du 11/10/2026. Définit **le jeu**, pas sa
> réalisation : ni ticket, ni maquette, ni code.

## 1. En une phrase

> Un RPG tactique multijoueur, joué à distance et en direct, à l'esprit
> DnD, dont l'histoire est préparée par un Auteur et racontée par le jeu.

```
 ┌──────────────────────────────────────────────────────┐
 │ Esprit       DnD 5e (classes, races, d20, jets)      │
 │ Narration    le jeu raconte et fait parler les PNJ   │
 │ Combat       règles DnD, lisibilité Fire Emblem      │
 │ Visuel       vrai HD-2D (Octopath, Triangle Strategy)│
 │ Plateforme   PC (clavier/souris + manette)           │
 └──────────────────────────────────────────────────────┘
```

**Inspirations**

| Jeu | Ce qu'on lui prend |
|---|---|
| Baldur's Gate 3 | esprit DnD dans un jeu programmé, narrateur, dialogues à choix avec jets visibles |
| Fire Emblem | combat tactique sur grille, aperçu avant l'attaque, portraits, banderoles de critique |
| Octopath Traveler | HD-2D, un arc personnel par héros |
| Triangle Strategy | HD-2D + tactique sur grille, combat dans le décor |
| Into the Breach | intentions ennemies visibles |

## 2. Les rôles

On passe d'un DnD avec un MJ au centre à un DnD « à la Baldur's Gate ».

```
 AVANT                                  PENDANT
 ─────                                  ───────
 Auteur + IA                            Jeu : narre, fait parler les PNJ, arbitre
   prépare l'histoire                     │
   et toutes ses options                  ▼
                                        Joueurs (dont l'Auteur)
                                          ▲
                                        MJ en retrait : override invisible
                                        Entre chapitres : réécriture via Claude + MCP
```

| Rôle | Fait quoi |
|---|---|
| **Auteur** | Prépare la campagne avec l'IA. Il joue aussi et n'est pas surpris (assumé). |
| **Jeu** | Applique les règles, raconte, fait parler les PNJ dans le cadre préparé. |
| **Joueurs** | Explorent librement, dialoguent, combattent. |
| **MJ** | Rôle réduit : il débloque et corrige en direct, et réécrit la suite entre deux chapitres. |

**Principe : structure figée, voix vivante.** Tout ce qui *peut arriver*
est préparé à l'avance. L'IA ne fait que *raconter* ces branches. Elle
n'invente jamais de branche qui casserait l'histoire.

## 3. La boucle de jeu

```
   ┌──▶ HUB (libre, chacun de son côté) ──▶ FIL (groupe réuni) ──┐
   │    PNJ · quêtes secondaires ·          révélation · combat  │
   │    mini-jeux · fouille                                      │
   └──────────────────── Jalon (niveau +1) ◀─────────────────────┘
```

- **Hub** : un lieu rempli (taverne, ville, camp). Chacun explore seul.
- **Fil** : l'histoire avance, tout le groupe est réuni.
- **Convergence** (hub → fil) : un événement préparé, tous les joueurs
  prêts, ou un joueur qui déclenche le fil. Le MJ peut aussi la forcer.

**Savoir privé.** Chaque joueur a son journal. Ce qu'il découvre, il le
garde ou le partage. Deux joueurs ne voient pas forcément la même chose
(voir la Perception, §6).

## 4. L'échelle

```
 Campagne courte : 4 à 6 chapitres, niveaux 1 à 5      (cible)
 One-shot : 1 chapitre                                 (format de test)

 Chapitre = une soirée, tout le monde connecté en même temps
   Hub ──▶ Fil ──▶ Combat clé ──▶ Jalon
```

Pas d'asynchrone. Chaque soirée a un début et une fin nets.

## 5. Règles

### Personnages
- Le joueur choisit **race, classe, historique**. Les **stats sont fixées
  par la classe** (tableau standard réparti au mieux pour elle).
- **Une classe = un seul joueur**, premier arrivé, premier servi, visible dans le lobby.
- L'Auteur peut **limiter les classes** disponibles dans sa campagne.
- **Prétirés** préparés par l'Auteur, en option.
- L'**historique** ouvre un **arc personnel** : une quête préparée par
  l'Auteur pour chaque historique.

### Chaque stat sert partout
Chaque joueur doit trouver son compte, en combat **et** hors combat.

| Stat | En combat | Hors combat |
|---|---|---|
| FOR | dégâts au corps à corps, pousser d'une case | bras de fer, forcer une porte |
| DEX | initiative, CA, armes à distance | crochetage, tricher aux dés, discrétion |
| CON | PV, concentration | concours de boisson, résister au poison |
| INT | sorts de magicien, faiblesses ennemies | énigmes, indices, histoire des lieux |
| SAG | intentions ennemies, éviter les embuscades | indicateurs cachés, détecter les mensonges |
| CHA | sorts de barde/ensorceleur, démoraliser | marchandage, persuasion, intimidation |

**Règle d'Auteur :** chaque hub offre **au moins une occasion par stat**.
La génération le vérifie.

### Progression
- **Jalons** : tout le groupe monte de niveau ensemble, à des moments clés.
- **Butin** : objets uniques préparés par l'Auteur + tables aléatoires.
  Chacun garde ce qu'il trouve, et peut le donner à un joueur proche.

### Échec
- **Perso à 0 PV** : à terre, jets contre la mort, un allié peut le relever.
  La mort est possible, et la résurrection rare et coûteuse.
  **La difficulté est réglable par l'Auteur.**
- **Perso mort** : son joueur prend un compagnon, ou crée un nouveau perso
  qui arrive au prochain hub. Il ne reste jamais spectateur.
- **Défaite du groupe** : un **échec qui fait avancer**. C'est une branche
  préparée (capturés, laissés pour morts…), jamais un game over.

## 6. Exploration

- Déplacement **libre** (pas de cases), clavier/souris ou manette.
- Les autres joueurs sont visibles dans le même lieu.
- Indicateurs au-dessus des PNJ et des objets : `[!]` quête, `[?]` mini-jeu.
  Certains sont **cachés** et n'apparaissent que si la **Perception
  passive** est suffisante.

## 7. Dialogue

```
 ┌────────┐  GORVIN, tavernier
 │portrait│  « On ne pose pas de questions, ici. »
 └────────┘
  💡 Perspicacité : il ment, il a peur de quelqu'un.

  1. « Je cherche une chambre. »
  2. [Persuasion]   « Je saurai être discret. »   DD 12
  3. [Perspicacité] « De qui as-tu peur ? »       (débloqué)
```

- Portraits façon Fire Emblem, avec quelques expressions.
- **Choix uniquement**, pas de texte libre. Jets visibles avec leur DD, et
  le d20 roule à l'écran.
- Certaines options ne sont **débloquées que par les stats**.
- On peut **rejoindre** une conversation : celui qui l'a lancée choisit,
  celui qui a rejoint voit tout et peut « souffler » un choix.

## 8. Combat

Le combat se déroule **sur place**, dans le décor de l'exploration, et alterne trois plans :

```
 1. TACTIQUE (vue de dessus) ──▶ 2. COUP (HD-2D) ──▶ 3. BANDEROLE (rare)
    même scène 3D, caméra haute     la caméra plonge     critique, sort ultime :
    grille, portées, choix          sur l'action         portrait « en action »
```

- **Initiative DnD**, affichée en frise. Pas de phases à la Fire Emblem.
- Ressources toujours visibles : Action, Action bonus, Réaction, Déplacement.
- **Aperçu avant de valider** : % de toucher et dégâts. On peut annuler tant que l'action n'est pas validée.
- **60 secondes par tour**. Une fois le temps écoulé, le perso prend l'action Esquiver.
- **Intentions ennemies** visibles selon la **Perspicacité**.
- Animations : celles des joueurs sont jouées, celles des ennemis sont
  accélérées ou désactivables, et la banderole est toujours jouée.

## 9. Mini-jeux

- Un **catalogue de types programmés**, que l'Auteur et l'IA habillent pour chaque lieu.
- Premiers types : **dés · crochetage · marchandage**.
- Uniquement dans les hubs, toujours optionnels, moins de 2 minutes.
- La compétence DnD liée change la difficulté (exemple : l'Escamotage permet de tricher aux dés).
- Gains : or, objet, rumeur, accès à une quête secondaire.
- Possibles entre joueurs (les dés en duel).

## 10. Écrans joueur

```
 Lobby ─▶ Création ─▶ ┌─ EN JEU ───────────────────────────┐
                      │ Exploration ◀─▶ Dialogue           │
                      │ Combat · Mini-jeu                  │
                      │ [Journal · Inventaire · Fiche]     │
                      └────────────────────────────────────┘
```

- **Lobby** : lien d'invitation, choix des classes en direct, résumé IA du
  chapitre précédent. Le MJ lance la partie.
- **Création** : race → classe → historique → (stats auto) → apparence
  (sprite PixelLab + portrait IA à partir d'une description) → nom.
  Chaque choix montre ce qu'il apporte, en combat et hors combat.
- **Journal** (privé) : quêtes, découvertes, personnes, notes du joueur,
  onglet Groupe (ce que les autres ont partagé), bouton Partager. Il est
  rempli automatiquement par le jeu.
- **Inventaire** : grille, comparaison ▲▼, Équiper / Donner (à un joueur proche) / Jeter.
- **Fiche** : chaque stat montre son effet concret, en combat et hors combat.

## 11. Écran MJ

Un **panneau** que le MJ ouvre par-dessus son propre écran de joueur (touche `Tab`).

- **Voir** : où est chaque joueur et ce qu'il fait, tous les journaux.
- **Alertes** : joueur bloqué, conversation qui tourne en rond, temps qui dérape.
- **Override** (**invisible** pour les joueurs) : pause globale,
  forcer/annuler un jet ou une action, déplacer un perso, déclencher la
  convergence.
- **Entre chapitres** : réécriture de la suite avec Claude via le MCP.

## 12. Direction artistique et contenu

- **Vrai HD-2D** : décor 3D, sprites en pixel art, lumière dynamique, flou de profondeur.
- **PixelLab** : sprites animés, tuiles, décors.
- **IA image** : portraits des PNJ et des persos, portraits « en action »
  pour les banderoles, illustrations des scènes clés.

**Ce que prépare l'Auteur** : moins un scénario linéaire que des **lieux
remplis**. Pour chaque hub, il faut des PNJ et leurs dialogues à choix,
des quêtes secondaires, des mini-jeux habillés, une occasion par stat, les
branches d'échec et la convergence vers le fil.

## 13. Écarts avec la vision précédente

`CLAUDE.md` a été mis à jour en conséquence.

| Règle précédente | Dans cette bible |
|---|---|
| 1. Le MJ a le dernier mot, rien n'atteint les joueurs sans validation | Le jeu narre seul dans un cadre préparé. Le MJ intervient seulement par override. |
| 2. L'IA en prépa et en direct, toujours derrière une validation | En direct, l'IA raconte sans validation, mais ne sort jamais de la structure préparée. |
| 6. Le téléphone d'abord pour les joueurs | Le PC d'abord (mis de côté pour l'instant). |

Les règles 3 (règles claires), 4 (pas de mock) et 5 (le français d'abord) restent inchangées.

## 14. Hors champ (à définir plus tard)

- Ambiance sonore (musique, effets).
- Choix du moteur 3D pour le HD-2D (dans le navigateur, ou un vrai moteur de jeu).
- Et tout ce que les premières parties de test feront apparaître.
