# Combat de vaisseau — Le Brasier (v1)

> Système de combat spatial pour le HMS-230 132 « Le Cure-Dent ».
> **Statut : v1 à playtester.** Les valeurs chiffrées sont des points de départ, à ajuster en jeu.

---

## 1. Principe

En combat spatial, le vaisseau est un **personnage partagé**. Chaque joueur tient un **poste** ou se déplace dans le vaisseau, et l'équipage fait avancer le combat collectivement, tour par tour.

**Deux façons de vaincre un vaisseau ennemi :**
- réduire sa **Coque** à 0 (on le détruit), **ou**
- briser son **Moral** à 0 (il fuit, se rend ou décroche).

---

## 2. Économie d'actions

Chaque joueur dispose de **2 actions par tour** (à bord du vaisseau ou au sol).

**Règles d'or :**
- **1 seule attaque par tour** maximum (tir de tourelle, canon lourd, ou « briser le Moral » comptent comme l'attaque).
- **Entrer ou sortir d'une tourelle / changer de poste = 1 action.**
- Une action peut être : agir à un poste, se déplacer, une action manuelle, utiliser un objet.
- Certaines actions **lourdes coûtent 2 actions** (tir chargé du canon, réparation majeure, manœuvre risquée) — elles consomment donc tout le tour.

> Exemple : un artilleur déjà en tourelle → **action 1** : tir (son attaque) ; **action 2** : se braquer, recharger, ou filer vers une autre tourelle. Un artilleur hors tourelle → **action 1** : entrer en tourelle ; **action 2** : tir.

---

## 3. Tour de jeu & initiative (quand les ennemis agissent)

À l'ouverture du combat, **chaque vaisseau** lance l'**initiative** : `d20 + modificateur DEX du pilote`. On joue du plus haut au plus bas, ordre fixe pour tout le combat.

- Quand vient le tour du **Cure-Dent**, **tout l'équipage agit** : chaque joueur prend ses 2 actions, dans l'ordre que la table décide (idéal pour coordonner : verrouiller *puis* tirer).
- Chaque **vaisseau ennemi** agit à son propre rang d'initiative (déplacement + attaque selon sa fiche).
- Les **escouades** de petits chasseurs peuvent partager une seule initiative et agir en groupe (voir §9).

**Début du tour du Cure-Dent** : on résout d'abord les **avaries en cours** (un incendie inflige ses dégâts, etc., voir §8).

---

## 4. La grille & les arcs de tir

Le combat se joue sur une **grille** (cases ou hexagones). Chaque vaisseau est un pion avec une **proue** (l'avant) — son orientation compte.

**Les 4 arcs**, définis par l'orientation de la proue :

```
        AVANT
          ▲
  BÂBORD ◄ ● ► TRIBORD
          ▼
        ARRIÈRE
```

Une arme ne peut tirer que sur une cible **située dans son arc**. D'où l'intérêt de manœuvrer pour « présenter » le bon flanc, ou de courir vers une autre tourelle.

**Portée**, mesurée en cases entre les deux vaisseaux :
- **0–2 cases** : courte (permet l'abordage)
- **3–5 cases** : moyenne
- **6–8 cases** : longue
- **9+** : hors de portée

---

## 5. Armement du Cure-Dent

| Arme | Poste (stat) | Arc | Portée | Dégâts | Notes |
|------|--------------|-----|--------|--------|-------|
| **Canon lourd « Pic-Vert »** | Pièce lourde (FOR) | **Avant uniquement** | Longue | **8** | Tir chargé possible : 2 actions → 12 dégâts. Il faut pointer la proue. |
| **Tourelle dorsale** | Tourelle (DEX) | **Avant + flancs** (angle mort arrière) | Moyenne | 4 | Large couverture mais frappe léger. |
| **Tourelle bâbord** | Tourelle (DEX) | Bâbord + Avant | Moyenne | 5 | |
| **Tourelle tribord** | Tourelle (DEX) | Tribord + Avant | Moyenne | 5 | |

Le dilemme tactique : la tourelle dorsale couvre l'avant et les flancs mais tape peu ; les tourelles de flanc frappent plus fort mais exigent d'**amener l'ennemi sur le bon côté** (manœuvre du pilote, ou un joueur qui change de tourelle) ; le canon lourd écrase mais **uniquement vers l'avant**.

> **Angle mort arrière** : aucune arme ne couvre le pur arrière du vaisseau (la coque et les nacelles bloquent le tir). Un ennemi qui se cale dans votre dos est **intouchable** — ne le laissez jamais s'y installer. C'est le pilotage qui vous en sort.

**Toucher** : `d20 + stat du poste (+ bonus de verrouillage)` vs **Blindage** de la cible. Les dégâts touchent d'abord les **Boucliers**, puis la **Coque**.

---

## 6. Caractéristiques du Cure-Dent (départ)

- **Coque** : 30
- **Boucliers** : 12 (rechargeables, voir Ingénieur)
- **Blindage (CA)** : 14
- **Énergie réacteur** : **6 points / tour** à répartir (voir ci-dessous)
- **Moral** : sans objet (un équipage de joueurs ne « fuit » pas mécaniquement)

Rappel : le Cure-Dent est **en avance technologique** — Blindage et Boucliers élevés lui permettent de tenir face à une escouade. Mais il est **irremplaçable** : chaque avarie compte.

### Énergie & répartition

Chaque tour, le réacteur fournit **6 points d'énergie** à répartir entre trois canaux : **NAVIGATION**, **ARMES**, **BOUCLIERS**. La répartition équilibrée de départ est **2 / 2 / 2**. Pour booster un canal, il faut en **affamer un autre**.

Le **Mécano** gère la distribution : elle tient jusqu'à ce qu'il la change. **Rerouter l'énergie = 1 action** (mise en place initiale gratuite au début du combat).

| Pts | NAVIGATION | ARMES | BOUCLIERS |
|-----|-----------|-------|-----------|
| **0** | Moteurs morts : ni déplacement ni esquive | Secours : -2 dégâts, pas de tir chargé | Hors ligne : plus de recharge, n'absorbe plus (dégâts → Coque) |
| **1** | Ralenti : -1 case | -1 dégât | Faible : recharge +2 |
| **2** | Nominal | Nominal | Nominal : recharge +4 |
| **3** | +1 case de déplacement | +1 dégât / tir | Renforcés : recharge +6 |
| **4** | +2 cases et +2 à l'esquive | +2 dégâts / tir | Surchargés : recharge +6 et -2 dégâts subis |

**Un vaisseau endommagé produit moins d'énergie :**
- Coque ≤ 20 → réacteur à **5 points**
- Coque ≤ 10 → réacteur à **4 points**
- Avarie « réacteur touché » → **-2 points** jusqu'à réparation

→ Plus la Coque souffre, plus les choix deviennent cruels : garder les boucliers, ou les canons chauds ?

---

## 7. Les postes & toutes les actions de bord

Chaque action ci-dessous coûte **1 action** sauf mention (2 actions).

### Pilote — barre (DEX)
- **Manœuvre** : avancer jusqu'à **3 cases** et pivoter d'un quart de tour (changer de proue).
- **Esquive** : +3 au Blindage du vaisseau jusqu'à son prochain tour.
- **Changer de portée** : se rapprocher (vers l'abordage) ou s'éloigner.
- **Manœuvre risquée** *(2 actions)* : jet DEX (DD 12) → double déplacement, prise à revers (l'ennemi perd son arc), ou semer un poursuivant.

### Pièce lourde & pont (FOR)
- **Tir du canon lourd** *(attaque)* : 8 dégâts, arc avant, longue portée.
- **Tir chargé** *(2 actions, attaque)* : 12 dégâts.
- **Repousser les abordeurs** : combat au sol dans la coursive (voir §10).
- **Éjecter un module / forcer un système grippé** : action de force (jet FOR).

### Ingénieur (INT)
- **Recharger les boucliers** : +4 Boucliers (max 12).
- **Rerouter l'énergie** : redistribue les points entre NAVIGATION / ARMES / BOUCLIERS (voir §6 — Énergie & répartition).
- **Réparer la coque** : jet INT (DD 10) → +3 Coque.
- **Réparer un système** hors service (voir avaries) : jet INT (DD 12).
- **Surcharger un système** *(2 actions)* : effet doublé ce tour, mais risque d'avarie (jet INT raté = incendie).

### Intégrité & avaries (CON)
- **Se braquer** : le vaisseau subit **-4 dégâts** sur toutes les attaques reçues jusqu'à son prochain tour.
- **Éteindre un incendie** : jet CON (DD 10).
- **Colmater une brèche** : jet CON (DD 12).
- **Maintenir un système en survie** : garde un poste en ligne malgré l'avarie, le temps qu'on le répare.

### Capteurs (SAG) — *tenu par LUMEN en session 1*
- **Verrouiller une cible** : la prochaine attaque alliée sur cette cible gagne **+4 à toucher**.
- **Scanner** : révèle Coque/Moral/point faible d'un vaisseau ennemi.
- **Brouiller** : une cible subit **-3 à toucher** jusqu'à son prochain tour.
- **Détecter** : repérer renforts, sorties de combat, dangers du décor.

### Liaison (CHA)
- **Briser le Moral** *(attaque)* : `d20 + CHA` vs Moral/SAG de l'ennemi → -X Moral (à 0, il décroche).
- **Bluffer** : faux transpondeur, fausse allégeance, faux signal de détresse → l'ennemi subit un malus ou perd une action.
- **Rallier l'équipage** : un allié relance un jet raté ou gagne +3.
- **Couper les comms ennemies** : empêche l'appel de renforts / la coordination de l'escouade.

### Déplacement & divers (toute classe)
- **Se déplacer dans le vaisseau** (rejoindre un poste, une tourelle, une section) : 1 action.
- **Entrer / sortir d'une tourelle** : 1 action.
- **EVA / sortir sur la coque** : réparations externes ou repositionnement risqué (jet DEX/CON).
- **Utiliser un objet.**

---

## 8. Avaries (le feu qui se déclenche)

Quand le Cure-Dent encaisse un **coup critique** (l'ennemi fait 20 naturel), **ou** quand sa Coque franchit les seuils **20** puis **10**, on lance **1d6 d'avarie** :

| 1d6 | Avarie | Effet | Réparation |
|-----|--------|-------|-----------|
| 1–2 | **Incendie** | 2 dégâts à la Coque au début de chaque tour ; peut se propager | Éteindre — jet CON (DD 10) |
| 3–4 | **Brèche** | Décompression : la section devient dangereuse (dégâts aux persos présents) | Colmater — jet CON (DD 12) |
| 5 | **Système touché** | Un poste (tourelle, moteur, boucliers, capteurs) **hors service** | Réparer — jet INT (DD 12) |
| 6 | **Secousse** | L'équipage est ballotté : un perso au hasard perd 1 action au prochain tour | — |

Les avaries donnent du travail à l'Ingénieur et au poste Intégrité — et créent des **crises** à gérer en plein combat.

---

## 9. Ennemis & escouades (fiches de référence)

| Vaisseau | Coque | Blindage | Boucl. | Moral | Armes | Notes |
|----------|-------|----------|--------|-------|-------|-------|
| **Chasseur Vorr** | 6 | 12 | 0 | *immunisé* | Crocs (3 dég.) | Rapide ; attaque en **essaim** (3-5, initiative partagée) |
| **Corvette pirate** | 15 | 13 | 6 | 6 | Canon (5 dég.) | Standard ; brise au Moral |
| **Carapace Sereth** | 25 | 16 | 10 | 8 | Mortier (6 dég.) | Très défensive, lente |

**Escouades** : un groupe de petits chasseurs partage une initiative et agit ensemble. Le **tir de barrage** ou une **tourelle 360°** sont efficaces contre eux. Couper leurs comms (Liaison) empêche leur coordination.

---

## 10. Abordage

À **courte portée (0-2 cases)**, on peut **arrimer** un vaisseau (ou se faire arrimer). Le combat **bascule alors en combat au sol** (voir `Combat_Sol.md`) : sas, coursives, salle des machines. Le poste **Pièce lourde (FOR)** mène la défense quand on repousse des abordeurs.

---

## 11. Fin du combat

- **Coque ennemie à 0** → vaisseau détruit (ou capturable si on a visé les moteurs).
- **Moral ennemi à 0** → reddition, fuite ou décrochage (et opportunité diplomatique : un ennemi épargné peut devenir un contact).
- **Le Cure-Dent décroche** → le Pilote réussit une manœuvre de fuite à longue portée.

> Rappel d'univers : détruire coûte un vaisseau irremplaçable adverse mais ferme une porte. **Briser le Moral** laisse une ouverture politique — souvent plus précieux dans le Brasier.

---

## 12. Pistes pour plus tard (v2)

- Énergie comme ressource chiffrée à répartir entre postes
- Modules & améliorations à looter/acheter pour customiser le Cure-Dent
- Manœuvres signatures par classe
- Cartes de décor spatial (astéroïdes, débris, champs de gaz, gravité)
