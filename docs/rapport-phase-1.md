# Rapport de la phase 1 — les règles à l'épreuve

Produit par `bun run rules-report` (`shared/src/bin/rules_report.rs`). Pour chaque monde : ce que le contrôle trouve dans le brouillon de règles, les chiffres d'équilibre, les combats simulés de ses scénarios (`content/scenarios/`), puis l'effet de chaque variante de règle, jouée sur les mêmes graines. Rien ici ne bloque : le MJ décide.

**Lire les simulations.**

- Les deux camps sont joués par des tactiques de référence, pas par une IA. *Bagarreur* : soigne un allié à terre, sinon frappe l'ennemi le plus proche, sinon avance droit sur lui. *Concentré* : pareil, mais frappe l'ennemi le plus faible à sa portée, se place à la case la moins chère d'où il peut tirer, et ne s'arrête jamais dans une porte. Les adversaires jouent en bagarreurs, avec le moral que le scénario leur donne (qui fuit, quand).
- Durée estimée (INTERPRÉTATION, à recaler sur un combat chronométré) : 2 min de mise en place, 60 s par tour de personnage, 30 s par tour d'adversaire, 5 s par tour vide (KO, rien à faire).
- Les chiffres par groupe sont des moyennes par combat ; un groupe d'adversaires additionne ses membres (5 marins = un groupe). « Touche » = jets d'attaque réussis sur jets d'attaque lancés (— : aucune attaque, la classe n'a que du soin ou du soutien à ce niveau).

## Le Brasier — règles `brasier` v1

Fichier : `content/rules/brasier/v1.yaml`.

### Contrôle des règles

**Erreurs (1)**

- `REFERENCE_MISSING` · `turn_contexts[vaisseau].references[0]` — Les règles renvoient à `Combat_Sol.md`, qui ne fait pas partie des documents du système. Écrivez-le et ajoutez-le aux sources, ou retirez le renvoi.

**Avertissements (2)**

- `PRECISION_NOT_APPLIED` · `attack.precision` — « Précision » n'est pas définie : 8 cartes ou états en donnent (Tir réflexe, Tir en mouvement, Tir de barrage, Arc électrique…), mais elle ne change aucun jet. Dites si elle s'ajoute au jet d'attaque, ou retirez-la des cartes.
- `PROGRESSION_MAX_EARLY` · `progression.levels` — L'XP s'emballe : Pilote, Mécano atteignent le niveau 7 avant la fin d'une campagne de 4 sessions (Pilote dès la session 3, environ 14,3 XP en session 1). Espacez les paliers, ou donnez moins d'XP par jet réussi.

**À confirmer (3)**

- `COOLDOWN_UNEXPLAINED` · `cooldowns.note` — Les recharges sont lues comme « inutilisable pendant vos N prochains tours (recharge 1 = un tour sur deux) », mais aucune phrase ne le dit aux joueurs. Écrivez-la dans `cooldowns.note`.
- `TURN_CONTEXTS_DIFFER` · `turn_contexts[vaisseau].limits[0]` — Combat de vaisseau : au plus 1 « Attaquer » par tour, contre 2 en « Combat au sol ». Confirmez que c'est voulu et dites-le aux joueurs.
- `DAMAGE_PER_TURN_LOW` · `classes[canonnier].actions` — Canonnier inflige environ 1,4 dégâts par tour au niveau 1 contre une CA de 10, moins de la moitié des autres classes offensives (4,2). Voulu ?

### Équilibre (calcul attendu, pas une simulation)

Campagne type : 4 sessions de 2 combats de 4 rounds et 6 jets hors combat ; contexte `sol` (2 actions par tour). Cibles : CA médiane des classes (CA 10). XP calculée contre CA 10, jets contre 10 ; total de caractéristiques médian 64.

| Classe | Total caract. | Dégâts/tour niv. 1 (CA médiane des classes) | Dégâts/tour niv. 7 | XP en fin de campagne | Niveau final | Niveau max dès |
|---|---:|---|---|---:|---:|---|
| Pilote | 64 | 4,2 | 5,0 | 57 | 7 | session 3 |
| Canonnier | 64 | 1,4 | 2,7 | 29 | 6 | jamais |
| Mécano | 64 | 4,2 | 4,4 | 57 | 7 | session 3 |
| Xénologue | 64 | 0,0 | 0,0 | 17 | 4 | jamais |
| Toubib | 64 | 0,0 | 0,0 | 16 | 4 | jamais |
| Quartier-maître | 64 | 0,0 | 0,0 | 17 | 4 | jamais |

### Combats simulés

#### L'abordage de la coursive

Six membres d'équipage de niveau 1 contre une escouade d'abordage vorr entrée par le sas bâbord.

Scénario `content/scenarios/brasier/abordage-coursive.yaml`, carte `cure-dent-coursive`, règles `brasier` v1 ; source : dnd-save/DnD_07-06-2026/Plan_Cure-Dent.svg (coursive, sas bâbord). 200 combats par tactique, graines 1 à 200.

> **Fiches d'adversaire hors règles.** Le système de règles n'a pas `abordeur-vorr`, `chef-d-escouade-vorr` : le scénario les définit lui-même, sans toucher au fichier de règles (voir le commentaire du scénario pour leur origine et ce qui n'est pas simulé).

| Tactique des PJ | Victoires PJ | Défaites | Nuls ou arrêtés | Rounds moy. (min–max) | Minutes estimées moy. (min–max) | Refus du moteur |
|---|---:|---:|---:|---|---|---:|
| bagarreur | 86 % (173/200) | 27 | 0 | 4,9 (1–17) | 30 (10–90) | 0 |
| concentré | 100 % (199/200) | 1 | 0 | 3,5 (1–7) | 24 (10–40) | 0 |

**Par groupe — PJ bagarreur** (tours par combat : 16,7 de PJ, 19,6 d'adversaires, 12,0 vides)

| Groupe | Nb | Dégâts infligés | Dégâts reçus | Touche | KO | Hors scène | Fuites | Vaincus | XP |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| Pilote | 1 | 6,8 | 4,4 | 52 % | 0,43 | 0,15 | 0,00 | 0,00 | 2,7 |
| Canonnier | 1 | 2,8 | 8,9 | 51 % | 0,76 | 0,12 | 0,00 | 0,00 | 0,8 |
| Mécano | 1 | 6,2 | 5,2 | 53 % | 0,55 | 0,16 | 0,00 | 0,00 | 2,5 |
| Xénologue | 1 | 0,0 | 5,2 | — | 0,49 | 0,12 | 0,00 | 0,00 | 0,0 |
| Toubib | 1 | 0,0 | 4,9 | — | 0,25 | 0,15 | 0,00 | 0,00 | 0,0 |
| Quartier-maître | 1 | 0,0 | 2,4 | — | 0,17 | 0,01 | 0,00 | 0,00 | 0,0 |
| Abordeur Vorr | 5 | 20,2 | 10,5 | 58 % | 2,34 | 0,00 | 2,13 | 2,34 | — |
| Chef d'escouade Vorr | 1 | 10,9 | 5,3 | 60 % | 0,33 | 0,00 | 0,54 | 0,33 | — |

**Par groupe — PJ concentré** (tours par combat : 14,3 de PJ, 14,2 d'adversaires, 4,7 vides)

| Groupe | Nb | Dégâts infligés | Dégâts reçus | Touche | KO | Hors scène | Fuites | Vaincus | XP |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| Pilote | 1 | 7,0 | 1,1 | 52 % | 0,04 | 0,01 | 0,00 | 0,00 | 2,9 |
| Canonnier | 1 | 2,6 | 5,9 | 51 % | 0,30 | 0,04 | 0,00 | 0,00 | 0,8 |
| Mécano | 1 | 6,7 | 2,0 | 52 % | 0,10 | 0,01 | 0,00 | 0,00 | 2,8 |
| Xénologue | 1 | 0,0 | 3,5 | — | 0,23 | 0,06 | 0,00 | 0,00 | 0,0 |
| Toubib | 1 | 0,0 | 4,0 | — | 0,18 | 0,07 | 0,00 | 0,00 | 0,0 |
| Quartier-maître | 1 | 0,0 | 1,0 | — | 0,04 | 0,00 | 0,00 | 0,00 | 0,0 |
| Abordeur Vorr | 5 | 11,8 | 13,5 | 59 % | 3,17 | 0,00 | 1,82 | 3,17 | — |
| Chef d'escouade Vorr | 1 | 5,8 | 2,8 | 56 % | 0,12 | 0,00 | 0,87 | 0,12 | — |

##### Variante : « CD 1 » = pas deux fois dans le même tour

Le brouillon lit « CD N » comme « inutilisable pendant vos N prochains tours » (CD 1 = un tour sur deux). L'autre lecture : le tour d'usage compte (CD 1 = pas deux fois dans le même tour).

Modifications appliquées au brouillon, en mémoire :

- `cooldowns.meaning` → `turn_of_use_counts`

Contrôle des règles : disparaît `DAMAGE_PER_TURN_LOW classes[canonnier].actions` ; apparaît aucun.

| Tactique des PJ | Victoires PJ (brouillon → variante) | Rounds moy. | Minutes moy. |
|---|---|---|---|
| bagarreur | 86 % → 90 % (+3 pts) | 4,9 → 4,7 (−0,3) | 30 → 29 (−0,9) |
| concentré | 100 % → 100 % (±0 pts) | 3,5 → 3,3 (−0,3) | 24 → 23 (−1,1) |

Écarts par groupe, PJ bagarreur (variante − brouillon, par combat) :

| Groupe | Dégâts infligés | Dégâts reçus | Touche | XP |
|---|---:|---:|---:|---:|
| Pilote | −0,14 | −0,13 | +2 pts | −0,06 |
| Canonnier | +1,41 | −0,92 | +1 pts | +0,40 |
| Mécano | −0,19 | −0,19 | +1 pts | −0,12 |
| Xénologue | ±0,00 | −0,33 | ±0 pts | ±0,00 |
| Toubib | ±0,00 | −0,35 | ±0 pts | ±0,00 |
| Quartier-maître | ±0,00 | −0,51 | ±0 pts | ±0,00 |
| Abordeur Vorr | −1,87 | +0,53 | +1 pts | — |
| Chef d'escouade Vorr | −0,56 | +0,55 | ±0 pts | — |

Écarts par groupe, PJ concentré (variante − brouillon, par combat) :

| Groupe | Dégâts infligés | Dégâts reçus | Touche | XP |
|---|---:|---:|---:|---:|
| Pilote | −0,70 | −0,11 | ±0 pts | −0,27 |
| Canonnier | +1,48 | −0,67 | ±0 pts | +0,48 |
| Mécano | −0,29 | −0,24 | ±0 pts | −0,26 |
| Xénologue | ±0,00 | −0,42 | ±0 pts | ±0,00 |
| Toubib | ±0,00 | −0,30 | ±0 pts | ±0,00 |
| Quartier-maître | ±0,00 | −0,22 | ±0 pts | ±0,00 |
| Abordeur Vorr | −1,34 | +0,34 | +2 pts | — |
| Chef d'escouade Vorr | −0,62 | +0,15 | ±0 pts | — |

##### Variante : La précision compte au jet d'attaque

Comme aux Corsaires, la précision des cartes n'entre pas au jet. Ici, elle s'ajoute au d20.

Modifications appliquées au brouillon, en mémoire :

- `attack.precision` → `added_to_attack_roll`

Contrôle des règles : disparaît `PRECISION_NOT_APPLIED attack.precision` ; apparaît aucun.

| Tactique des PJ | Victoires PJ (brouillon → variante) | Rounds moy. | Minutes moy. |
|---|---|---|---|
| bagarreur | 86 % → 90 % (+4 pts) | 4,9 → 4,6 (−0,3) | 30 → 28 (−1,8) |
| concentré | 100 % → 100 % (+1 pts) | 3,5 → 3,2 (−0,3) | 24 → 22 (−2,0) |

Écarts par groupe, PJ bagarreur (variante − brouillon, par combat) :

| Groupe | Dégâts infligés | Dégâts reçus | Touche | XP |
|---|---:|---:|---:|---:|
| Pilote | +0,68 | −0,39 | +12 pts | +0,30 |
| Canonnier | +0,25 | −1,07 | +7 pts | +0,06 |
| Mécano | +0,20 | −0,27 | +5 pts | +0,04 |
| Xénologue | ±0,00 | −0,25 | ±0 pts | ±0,00 |
| Toubib | ±0,00 | −0,42 | ±0 pts | ±0,00 |
| Quartier-maître | ±0,00 | −0,80 | ±0 pts | ±0,00 |
| Abordeur Vorr | −2,40 | +0,31 | ±0 pts | — |
| Chef d'escouade Vorr | −0,79 | +0,81 | ±0 pts | — |

Écarts par groupe, PJ concentré (variante − brouillon, par combat) :

| Groupe | Dégâts infligés | Dégâts reçus | Touche | XP |
|---|---:|---:|---:|---:|
| Pilote | +0,72 | −0,19 | +12 pts | +0,31 |
| Canonnier | +0,11 | −1,15 | +5 pts | +0,02 |
| Mécano | +0,19 | −0,30 | +6 pts | −0,04 |
| Xénologue | ±0,00 | −0,49 | ±0 pts | ±0,00 |
| Toubib | ±0,00 | −0,44 | ±0 pts | ±0,00 |
| Quartier-maître | ±0,00 | −0,36 | ±0 pts | ±0,00 |
| Abordeur Vorr | −2,14 | +0,45 | ±0 pts | — |
| Chef d'escouade Vorr | −0,78 | +0,57 | +1 pts | — |

## Corsaires de la Couronne — règles `corsaires` v1

Fichier : `content/rules/corsaires/v1.yaml`.

### Contrôle des règles

**Avertissements (14)**

- `DAMAGE_MODEL_MIXED` · `adversaries[gueule_rouge].actions[gueule_rouge_sabre].tags[0]` — Deux modèles de dégâts : les PNJ utilisent des dés (Gueule-Rouge 1d6+2, Marin de Gueule-Rouge 1d4+1, Garde royal 1d6+1, Capitaine Morel 1d6 et 5 autres) alors que les attaques des joueurs utilisent des dégâts fixes (ex. Estocade : 3). Un joueur ne sait plus lequel s'applique ; choisissez-en un seul.
- `DAMAGE_MODEL_MIXED` · `items[pistolet_a_silex_marche_noir].action.tags[0]` — Deux modèles de dégâts : les objets utilisent des dés (Pistolet à silex 1d8) alors que les attaques des joueurs utilisent des dégâts fixes (ex. Estocade : 3). Un joueur ne sait plus lequel s'applique ; choisissez-en un seul.
- `PRECISION_NOT_APPLIED` · `attack.precision` — « Précision » n'est pas définie : 12 cartes ou états en donnent (Estocade, Danse des lames, Salve de bordée, Lecture des vents…), mais elle ne change aucun jet. Dites si elle s'ajoute au jet d'attaque, ou retirez-la des cartes.
- `PRIMARY_ABILITY_AMBIGUOUS` · `attack.ability` — « Stat principale » est ambiguë : 6 classes en ont plusieurs (Bretteur : FOR ou DEX ; Canonnier : INT ou FOR ; Navigateur : SAG ou INT ; Vigie : DEX ou SAG ; Flibustier : FOR ou CON ; Boucanier : DEX ou CON). Le moteur prend la première de la liste ; dites laquelle compte, ou prenez la meilleure.
- `ADVERSARY_AC_OFF_TIER` · `adversaries[gueule_rouge].armor_class` — Gueule-Rouge a une CA de 12 alors que la table de référence donne 15 pour « Boss / Ennemi d'élite ». Alignez-la, ou marquez-la comme exception (`exception: <raison>`).
- `ADVERSARY_AC_OFF_TIER` · `adversaries[garde_royal].armor_class` — Garde royal a une CA de 14 alors que la table de référence donne 12 pour « Soldat entraîné ». Alignez-la, ou marquez-la comme exception (`exception: <raison>`).
- `ADVERSARY_AC_FORMULA` · `adversaries[jacquot_le_sourd].armor_class` — Jacquot le Sourd a une CA de 9 alors que la formule (10 + mod(DEX)) donne 8. Corrigez-la, ou marquez-la comme exception (`exception: <raison>`).
- `ADVERSARY_AC_FORMULA` · `adversaries[mere_goulven].armor_class` — Mère Goulven a une CA de 10 alors que la formule (10 + mod(DEX)) donne 9. Corrigez-la, ou marquez-la comme exception (`exception: <raison>`).
- `ADVERSARY_AC_FORMULA` · `adversaries[vieux_thomas].armor_class` — Vieux Thomas a une CA de 10 alors que la formule (10 + mod(DEX)) donne 9. Corrigez-la, ou marquez-la comme exception (`exception: <raison>`).
- `UNDEFINED_TERM` · `items[epee_de_bonne_facture].note` — Le texte compare à « épée standard », que le système ne définit nulle part. Définissez-le (objet, action…) ou donnez la valeur directement.
- `UNDEFINED_TERM` · `items[kit_de_soins].note` — Le texte parle de « jets de soin », mais aucun jet de ce genre n'existe dans le système. Dites ce que l'effet modifie vraiment.
- `NAME_DUPLICATE` · `items[pistolet_a_silex_marche_noir].name` — 2 objets s'appellent « Pistolet à silex » (pistolet_a_silex, pistolet_a_silex_marche_noir). Un joueur ne sait pas lequel il a ; renommez-en un ou fusionnez-les.
- `ABILITY_TOTAL_OUTLIER` · `classes[canonnier].abilities` — Canonnier totalise 63 points de caractéristiques, contre 64 pour les autres classes. Ajustez un score, ou dites pourquoi.
- `PROGRESSION_MAX_EARLY` · `progression.levels` — L'XP s'emballe : Bretteur, Vigie, Flibustier, Boucanier atteignent le niveau 7 avant la fin d'une campagne de 4 sessions (Bretteur dès la session 3, environ 11,9 XP en session 1). Espacez les paliers, ou donnez moins d'XP par jet réussi.

**À confirmer (1)**

- `DAMAGE_PER_TURN_LOW` · `classes[canonnier].actions` — Canonnier inflige environ 1,2 dégâts par tour au niveau 1 contre une CA de 12, moins de la moitié des autres classes offensives (3,6). Voulu ?

### Équilibre (calcul attendu, pas une simulation)

Campagne type : 4 sessions de 2 combats de 4 rounds et 6 jets hors combat ; contexte `sol` (2 actions par tour). Cibles : Matelot / pirate basique (CA 10), Soldat entraîné (CA 12), Officier / Capitaine (CA 13), Boss / Ennemi d'élite (CA 15). XP calculée contre CA 12, jets contre 10 ; total de caractéristiques médian 64.

| Classe | Total caract. | Dégâts/tour niv. 1 (Matelot / pirate basique / Soldat entraîné / Officier / Capitaine / Boss / Ennemi d'élite) | Dégâts/tour niv. 7 | XP en fin de campagne | Niveau final | Niveau max dès |
|---|---:|---|---|---:|---:|---|
| Bretteur | 64 | 3,9 / 3,3 / 3,0 / 2,4 | 4,0 / 3,4 / 3,1 / 2,5 | 48 | 7 | session 3 |
| Canonnier | 63 | 1,4 / 1,2 / 1,1 / 0,9 | 2,7 / 2,4 / 2,1 / 1,8 | 27 | 6 | jamais |
| Navigateur | 64 | 0,0 / 0,0 / 0,0 / 0,0 | 0,0 / 0,0 / 0,0 / 0,0 | 16 | 4 | jamais |
| Chirurgien de bord | 64 | 0,0 / 0,0 / 0,0 / 0,0 | 0,0 / 0,0 / 0,0 / 0,0 | 16 | 4 | jamais |
| Quartier-maître | 64 | 0,0 / 0,0 / 0,0 / 0,0 | 0,0 / 0,0 / 0,0 / 0,0 | 17 | 4 | jamais |
| Vigie | 64 | 4,2 / 3,6 / 3,3 / 2,7 | 5,0 / 4,4 / 4,2 / 3,6 | 51 | 7 | session 3 |
| Flibustier | 64 | 5,6 / 4,8 / 4,4 / 3,6 | 5,9 / 5,0 / 4,6 / 3,8 | 51 | 7 | session 3 |
| Boucanier | 64 | 4,2 / 3,6 / 3,3 / 2,7 | 6,5 / 6,0 / 5,7 / 5,1 | 51 | 7 | session 3 |

### Combats simulés

#### La bagarre du quai

Combat tutoriel de l'acte 1 : les six corsaires tombent sur Gueule-Rouge et ses marins entre les entrepôts et l'appontement.

Scénario `content/scenarios/corsaires/bagarre-du-quai.yaml`, carte `quai-port-louis`, règles `corsaires` v1 ; source : dnd-save/DnD-16-05-2026/Acte_1/acte_1.md (scène 4). 200 combats par tactique, graines 1 à 200.

| Tactique des PJ | Victoires PJ | Défaites | Nuls ou arrêtés | Rounds moy. (min–max) | Minutes estimées moy. (min–max) | Refus du moteur |
|---|---:|---:|---:|---|---|---:|
| bagarreur | 98 % (195/200) | 5 | 0 | 3,3 (2–7) | 23 (12–43) | 0 |
| concentré | 94 % (189/200) | 11 | 0 | 3,7 (2–10) | 24 (12–39) | 0 |

**Par groupe — PJ bagarreur** (tours par combat : 14,4 de PJ, 12,0 d'adversaires, 2,8 vides)

| Groupe | Nb | Dégâts infligés | Dégâts reçus | Touche | KO | Hors scène | Fuites | Vaincus | XP |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| Bretteur | 1 | 4,0 | 7,6 | 53 % | 0,57 | 0,06 | 0,00 | 0,00 | 1,5 |
| Canonnier | 1 | 3,1 | 4,3 | 55 % | 0,29 | 0,05 | 0,00 | 0,00 | 0,8 |
| Navigateur | 1 | 0,0 | 0,3 | — | 0,04 | 0,00 | 0,00 | 0,00 | 0,0 |
| Chirurgien de bord | 1 | 0,0 | 4,2 | — | 0,26 | 0,06 | 0,00 | 0,00 | 0,0 |
| Vigie | 1 | 8,5 | 0,5 | 60 % | 0,03 | 0,00 | 0,00 | 0,00 | 3,4 |
| Flibustier | 1 | 6,3 | 3,7 | 58 % | 0,27 | 0,07 | 0,00 | 0,00 | 1,8 |
| Gueule-Rouge | 1 | 8,3 | 9,9 | 58 % | 0,97 | 0,00 | 0,00 | 0,97 | — |
| Marin de Gueule-Rouge | 5 | 12,2 | 12,1 | 52 % | 2,60 | 0,00 | 2,30 | 2,60 | — |

**Par groupe — PJ concentré** (tours par combat : 16,1 de PJ, 11,2 d'adversaires, 3,1 vides)

| Groupe | Nb | Dégâts infligés | Dégâts reçus | Touche | KO | Hors scène | Fuites | Vaincus | XP |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| Bretteur | 1 | 4,3 | 8,3 | 52 % | 0,60 | 0,10 | 0,00 | 0,00 | 1,6 |
| Canonnier | 1 | 3,1 | 4,8 | 57 % | 0,34 | 0,08 | 0,00 | 0,00 | 0,9 |
| Navigateur | 1 | 0,0 | 0,6 | — | 0,06 | 0,00 | 0,00 | 0,00 | 0,0 |
| Chirurgien de bord | 1 | 0,0 | 4,1 | — | 0,25 | 0,12 | 0,00 | 0,00 | 0,0 |
| Vigie | 1 | 8,6 | 0,7 | 57 % | 0,06 | 0,03 | 0,00 | 0,00 | 3,5 |
| Flibustier | 1 | 8,1 | 3,6 | 61 % | 0,24 | 0,10 | 0,00 | 0,00 | 2,3 |
| Gueule-Rouge | 1 | 12,5 | 9,7 | 61 % | 0,94 | 0,00 | 0,00 | 0,94 | — |
| Marin de Gueule-Rouge | 5 | 9,6 | 14,4 | 52 % | 3,37 | 0,00 | 1,62 | 3,37 | — |

##### Variante : La précision compte au jet d'attaque

Les règles affichent une précision sur chaque carte mais ne l'ajoutent jamais au jet. Ici, elle s'ajoute au d20.

Modifications appliquées au brouillon, en mémoire :

- `attack.precision` → `added_to_attack_roll`

Contrôle des règles : disparaît `PRECISION_NOT_APPLIED attack.precision` ; apparaît aucun.

| Tactique des PJ | Victoires PJ (brouillon → variante) | Rounds moy. | Minutes moy. |
|---|---|---|---|
| bagarreur | 98 % → 98 % (+1 pts) | 3,3 → 3,0 (−0,3) | 23 → 21 (−1,7) |
| concentré | 94 % → 100 % (+5 pts) | 3,7 → 3,1 (−0,5) | 24 → 21 (−2,5) |

Écarts par groupe, PJ bagarreur (variante − brouillon, par combat) :

| Groupe | Dégâts infligés | Dégâts reçus | Touche | XP |
|---|---:|---:|---:|---:|
| Bretteur | +0,48 | −1,24 | +13 pts | +0,20 |
| Canonnier | −0,13 | −0,46 | +2 pts | −0,03 |
| Navigateur | ±0,00 | −0,20 | ±0 pts | ±0,00 |
| Chirurgien de bord | ±0,00 | −0,64 | ±0 pts | ±0,00 |
| Vigie | +0,38 | −0,23 | +13 pts | +0,35 |
| Flibustier | −0,14 | −0,28 | +4 pts | −0,07 |
| Gueule-Rouge | −0,89 | +0,09 | ±0 pts | — |
| Marin de Gueule-Rouge | −2,16 | +0,49 | +1 pts | — |

Écarts par groupe, PJ concentré (variante − brouillon, par combat) :

| Groupe | Dégâts infligés | Dégâts reçus | Touche | XP |
|---|---:|---:|---:|---:|
| Bretteur | +0,68 | −1,38 | +15 pts | +0,28 |
| Canonnier | −0,03 | −1,09 | +3 pts | −0,02 |
| Navigateur | ±0,00 | −0,55 | ±0 pts | ±0,00 |
| Chirurgien de bord | ±0,00 | −0,53 | ±0 pts | ±0,00 |
| Vigie | +0,67 | −0,63 | +17 pts | +0,41 |
| Flibustier | −0,64 | −0,73 | +2 pts | −0,18 |
| Gueule-Rouge | −2,86 | +0,31 | −2 pts | — |
| Marin de Gueule-Rouge | −2,06 | +0,37 | ±0 pts | — |

##### Variante : Un seul modèle de dégâts (PNJ à dégâts fixes)

Les joueurs font des dégâts fixes, les PNJ lancent des dés. Ici, Gueule-Rouge et ses marins font des dégâts fixes : la moyenne du dé, arrondie à l'inférieur (1d6+2 → 5, 1d4+2 → 4, 1d4+1 → 3, 1d3 → 2). Arrondir à l'inférieur leur retire un peu de dégâts moyens : la comparaison mêle donc la fin du hasard et cette petite baisse.

Modifications appliquées au brouillon, en mémoire :

- `adversaries[gueule_rouge].actions[gueule_rouge_sabre].tags[0].damage.amount` → `5`
- `adversaries[gueule_rouge].actions[gueule_rouge_coup_de_tete].tags[0].damage.amount` → `4`
- `adversaries[marin_de_gueule_rouge].actions[marin_coutelas].tags[0].damage.amount` → `3`
- `adversaries[marin_de_gueule_rouge].actions[marin_poing].tags[0].damage.amount` → `2`

Contrôle des règles : disparaît `DAMAGE_MODEL_MIXED adversaries[gueule_rouge].actions[gueule_rouge_sabre].tags[0]` ; apparaît `DAMAGE_MODEL_MIXED adversaries[garde_royal].actions[garde_epee].tags[0]`.

| Tactique des PJ | Victoires PJ (brouillon → variante) | Rounds moy. | Minutes moy. |
|---|---|---|---|
| bagarreur | 98 % → 99 % (+2 pts) | 3,3 → 3,1 (−0,2) | 23 → 22 (−0,7) |
| concentré | 94 % → 99 % (+5 pts) | 3,7 → 3,4 (−0,3) | 24 → 23 (−0,7) |

Écarts par groupe, PJ bagarreur (variante − brouillon, par combat) :

| Groupe | Dégâts infligés | Dégâts reçus | Touche | XP |
|---|---:|---:|---:|---:|
| Bretteur | +0,69 | −0,50 | +4 pts | +0,24 |
| Canonnier | +0,31 | −1,33 | +7 pts | +0,09 |
| Navigateur | ±0,00 | −0,20 | ±0 pts | ±0,00 |
| Chirurgien de bord | ±0,00 | −0,99 | ±0 pts | ±0,00 |
| Vigie | −0,78 | −0,37 | −1 pts | −0,28 |
| Flibustier | +0,20 | −0,66 | +2 pts | +0,03 |
| Gueule-Rouge | −1,05 | +0,12 | +4 pts | — |
| Marin de Gueule-Rouge | −3,01 | +0,30 | −4 pts | — |

Écarts par groupe, PJ concentré (variante − brouillon, par combat) :

| Groupe | Dégâts infligés | Dégâts reçus | Touche | XP |
|---|---:|---:|---:|---:|
| Bretteur | +0,84 | −0,28 | +6 pts | +0,29 |
| Canonnier | +0,38 | −1,19 | +4 pts | +0,09 |
| Navigateur | ±0,00 | −0,50 | ±0 pts | ±0,00 |
| Chirurgien de bord | ±0,00 | −0,53 | ±0 pts | ±0,00 |
| Vigie | −0,45 | −0,61 | +1 pts | −0,23 |
| Flibustier | −0,40 | −0,82 | −1 pts | −0,09 |
| Gueule-Rouge | −1,62 | +0,23 | −2 pts | — |
| Marin de Gueule-Rouge | −2,32 | +0,13 | −3 pts | — |

