# Parcours utilisateurs — Promptus

Ticket `design/map-user-journeys`. Ce document décrit ce que chacun
fait, dans quel ordre, et sur quel écran. Il sert à vérifier qu'aucun
écran ne manque avant de coder. La version visuelle est sur la planche
« Parcours » du canevas de design (lien dans `TICKETS.md`).

Légende des écrans : **dessiné** (planche existante dans le canevas),
**à dessiner** (manque encore).

## Les rôles

| Rôle | Appareil | Ce qu'il fait |
| --- | --- | --- |
| **MJ** | Ordinateur pour préparer, ordinateur ou tablette en direct | Prépare la campagne avec l'IA, la relit, mène la session, valide tout |
| **Joueur** | Téléphone (ordinateur possible) | Rejoint par un lien, joue son personnage, sans compte |
| **Écran partagé (TV)** | TV, ou partage d'écran (Discord…) | Affiche ce que tout le monde peut voir ; aucune interaction |
| **Co-MJ (IA)** | — | Propose des brouillons au MJ, jamais directement aux joueurs |

Rappels qui valent pour tous les parcours :

- Rien de généré n'atteint un joueur sans validation du MJ.
- Le serveur décide de chaque règle (déplacement, portée, attaque).
- Un joueur ne reçoit jamais ce qu'il n'a pas le droit de voir (notes
  MJ, cases non révélées, objets cachés, PV des adversaires).

---

## Parcours du MJ

### M1 · Créer une campagne

1. Le MJ se connecte (passkey) et arrive sur **la liste de ses
   campagnes**. · *dessiné* (Préparer)
2. « Nouvelle campagne » : il choisit **l'univers** (fantasy, zombies,
   spatial…), qui fixe le pack de tuiles, de personnages et d'objets, et
   les noms des statistiques. · *dessiné* (Préparer)
3. Il choisit **le système de règles** : un préréglage (D&D 5e SRD)
   qu'il peut modifier. · *dessiné* (Préparer)
4. Il écrit **le pitch** en quelques phrases (ton, durée, nombre de
   joueurs, budget IA). · *dessiné* (Préparer)

### M2 · Générer et relire l'histoire

1. La génération démarre : **suivi en direct** (étapes, coût engagé,
   budget restant). · *dessiné* (Préparer)
2. Le MJ relit **la bible, les fronts et leurs horloges, le graphe des
   scènes et des indices**, en discutant avec le LLM : il demande,
   l'IA propose un diff, il accepte ou refuse. · *dessiné* (Préparer,
   atelier)
3. Les alertes de cohérence s'affichent (une révélation avec moins de
   trois indices, un PNJ orphelin). · *dessiné*
4. Il relit **les fiches** : PNJ, monstres, objets et butin avec leur
   rareté. Il peut créer un PNJ avec le même créateur que les joueurs.
   · *dessiné* (Préparer, fiches ; objets, raretés)
5. Il **valide la campagne** : elle devient jouable. · *dessiné*
   (Préparer)

### M3 · Préparer les cartes

1. Pour chaque lieu du graphe, l'IA **propose une carte** dans le décor
   choisi (crypte, forêt, rue, désert…). · *à dessiner* (éditeur)
2. Le MJ la retouche dans **l'éditeur** : peindre murs et terrains,
   poser décors, portes, objets cachés, lumières, régler l'ambiance.
   · *à dessiner*
3. Ou il **importe une image** et aligne la grille ; l'IA propose les
   murs, il les valide. · *dessiné* (planche Cartes, vue MJ)
4. Il relie les échelles : **monde** en hexagones → **lieu** → **rencontre**.
   · *dessiné* (planches Cartes et Extérieurs)

### M4 · Préparer les médias

1. Pour chaque scène : image, vidéo d'introduction, musique YouTube.
   · *dessiné* (Préparer, cartes et médias)
2. Le coût est **estimé avant chaque lot** et comparé au budget ; les
   tâches tournent en arrière-plan. · *dessiné* (Préparer, cartes et médias)
3. Le MJ valide ou relance chaque média.

### M5 · Inviter les joueurs

1. Le MJ copie **le lien d'invitation** de la campagne. · *dessiné* (Inviter)
2. Il voit qui a rejoint, et **valide les personnages** créés par les
   joueurs (un personnage hors règles est renvoyé avec un mot).
   · *dessiné* (Inviter)
3. Il fixe la date de la prochaine session. · *dessiné* (Inviter)

### M6 · Lancer la session

1. **Le salon** : les joueurs arrivent, présence en direct, test du son.
   · *à dessiner*
2. Le MJ ouvre éventuellement **l'écran partagé** (code ou QR à saisir
   sur la TV, ou partage d'écran). · *à dessiner*
3. Il diffuse **« Précédemment… »** puis la première scène.
   · *dessiné* (notifications) · *à dessiner* (écran « Précédemment »)

### M7 · Mener la scène

La boucle principale, sur **l'écran MJ en direct**. · *dessiné*

1. Le MJ décrit la scène ; le co-MJ propose une narration qu'il modifie
   puis **montre aux joueurs**.
2. Les **demandes des joueurs** arrivent en cartes : il valide, refuse,
   ou demande un test de dé.
3. Il **révèle** : une zone de la carte, un indice, un objet caché.
4. Le co-MJ **fait parler un PNJ**, propose « et ensuite ? » ou les
   conséquences d'un choix ; tout reste un brouillon.
5. Les **horloges des fronts** avancent quand personne n'agit.
6. Entre deux lieux, le groupe **voyage** sur la carte du monde, par
   portions de journée. · *à dessiner* (écran de voyage)

### M8 · Mener un combat

1. Le MJ lance la rencontre : la carte de combat s'ouvre partout.
   · *dessiné*
2. **Initiative** calculée, ordre du tour affiché. · *dessiné*
3. Chaque tour, le serveur vérifie déplacement, portée et ligne de vue ;
   le MJ joue les adversaires ou laisse le co-MJ proposer. · *dessiné*
4. **États** posés et retirés, avec leurs tours restants. · *dessiné*
5. Fin du combat : le MJ **valide le butin**, il tombe sur la TV et
   dans les sacs. · *dessiné* (objets, notifications)
6. Expérience et **niveau supérieur** si besoin. · *dessiné*
   (notifications)

### M9 · Clore la session

1. « Terminer la session » : l'état de la campagne est enregistré.
   · *à dessiner*
2. L'IA rédige **le récapitulatif MJ** et **le « Précédemment… »
   joueurs** ; le MJ les relit et les publie. · *à dessiner*
3. La **chronique de campagne** s'allonge d'une entrée. · *à dessiner*

---

## Parcours du joueur

### J1 · Rejoindre

1. Il ouvre **le lien** reçu (sans compte, un jeton reste sur son
   téléphone). · *dessiné* (Inviter)
2. Il choisit : **créer son personnage**, **reprendre** le sien, ou
   **regarder** en spectateur. · *dessiné* (Inviter ; « reprendre » vu
   côté MJ seulement)

### J2 · Créer son personnage

1. **Le créateur en couches** : corps, peau, cheveux, tenue, arme,
   accessoire, couleurs, bouton « au hasard ». Les pièces viennent du
   pack de l'univers. · *à dessiner*
2. Il remplit ce que les règles demandent (classe, caractéristiques),
   guidé pas à pas. · *dessiné en partie* (Inviter : le choix de la
   classe, le renvoi avec le mot du MJ)
3. Il l'envoie au MJ et attend la validation. · *dessiné* (Inviter)

Objectif : moins de deux minutes, sur un téléphone.

### J3 · Entre deux sessions

1. Il lit **« Précédemment… »**. · *à dessiner*
2. Il consulte **sa fiche** et **son sac**, équipe un objet.
   · *dessiné* (sac) · *à dessiner* (fiche)

### J4 · Vivre la scène

Sur **l'écran scène** du téléphone. · *dessiné*

1. Le lieu, le texte lu par le MJ, l'image ; la musique démarre muette
   avec un lecteur visible et « activer le son ».
2. Les indices montrés par le MJ arrivent en carte.
3. **Que fais-tu ?** Il choisit une carte d'action (dérivée des règles)
   ou écrit une demande libre.
4. Le MJ répond : un test de dé à lancer, un résultat, ou un refus
   expliqué.
5. Le **journal** garde ce que tout le groupe sait. · *à dessiner*

### J5 · Explorer la carte

1. Sur l'onglet **Plateau**, il voit la carte révélée et son pion.
   · *dessiné*
2. Il glisse son pion : les cases atteignables sont surlignées, le
   serveur valide le trajet. · *dessiné*
3. Ce qui n'est pas vu reste dans le brouillard ; s'il est invisible,
   il se voit en fantôme. · *dessiné*

### J6 · Combattre

1. **« À toi, Borin ! »** tombe en tampon quand vient son tour.
   · *dessiné*
2. Il choisit une carte (Frappe), vise, lance le dé coloré de la stat ;
   le serveur résout. · *dessiné*
3. Le gros bouton arcade joue la carte choisie ; « Fin du tour » à côté.
   · *dessiné*
4. Il voit ses cœurs, ses ressources en cases et ses états.
   · *dessiné*

### J7 · Recevoir

Butin, niveau supérieur, compétence débloquée : une carte tombe sur
son téléphone, le grand moment passe sur la TV. · *dessiné*

### J8 · Après la session

Il lit le récapitulatif, la chronique, et retrouve son personnage
pour la prochaine fois. · *à dessiner*

---

## Écran partagé (TV)

1. Le MJ l'ouvre ; la TV affiche un code, ou il partage la fenêtre.
   · *à dessiner*
2. Elle montre la carte, la scène, l'ordre du tour, le groupe, les
   menaces, et joue les grands moments (dés, coups, butin, niveaux,
   révélations). · *dessiné*
3. Elle ne montre que ce que tous les joueurs ont le droit de voir.

---

## Écrans à dessiner

Classés par ordre de passage dans une campagne.

| Écran | Parcours | Appareil |
| --- | --- | --- |
| Éditeur du système de règles | M1 | Ordinateur |
| Éditeur de carte | M3 | Ordinateur |
| Invitation et validation des personnages | M5 | Ordinateur |
| Rejoindre (lien, choix) | J1 | Téléphone |
| Créateur de personnage | J2 | Téléphone |
| Fiche du personnage | J3 | Téléphone |
| Salon d'avant-session | M6 | Ordinateur et téléphone |
| Jumelage de l'écran partagé | M6 | TV |
| « Précédemment… » | M6, J3 | Téléphone, TV |
| Journal du groupe | J4 | Téléphone |
| Voyage sur la carte du monde | M7 | Ordinateur, téléphone, TV |
| Fin de session et récapitulatifs | M9, J8 | Ordinateur, téléphone |
| Joueur sur ordinateur | J4–J6 | Ordinateur |

## Questions ouvertes

1. **L'écran partagé** : `MEMORY.md` disait « table + TV plus tard » ;
   on l'a dessiné dès maintenant. Est-ce un écran ouvert sur une vraie
   TV, une fenêtre partagée sur Discord, ou les deux ?
2. **Création des personnages** : avant la première session (le MJ
   valide à froid), ou pendant une session zéro tous ensemble ?
3. **Entre les sessions** : le joueur peut-il faire quelque chose
   (équiper, gérer son sac, écrire au MJ), ou seulement lire ?
4. **Mort d'un personnage** : quel parcours (spectateur, nouveau
   personnage dans la session, attente) ?
5. **Spectateurs** : utiles (un ami qui regarde, un joueur absent qui
   suit), ou à retirer ?
