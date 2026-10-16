# Carte des docs

```text
 bible.md              la vision : le jeu qu'on fait, ses inspirations
 regles/jeu/           le livre du joueur : ce que tu peux faire, les dés, les issues
 regles/campagne/      le livre de l'Auteur : ce qu'il prépare et ce qu'il règle
          │ chaque règle est « résolue par » ▼
 plateforme/           comment le jeu le montre et le fait tourner (écrans, technique)
          │ chaque fiche liste ses « tickets » ▼
 tickets/              le travail, un fichier par ticket
```

- Une règle sans « Résolue par » est un **trou de plateforme** : les index le montrent.
- Une limite technique ne change jamais une règle en douce : elle remonte comme une règle à trancher.
- Quand une règle est muette, on suit la **référence par défaut** de son domaine (index de `regles/jeu/`).
- **Le test d'une phrase** : le joueur peut-il le faire, le savoir ou le risquer ? → règle. Comment c'est montré, construit ou cadré ? → fiche plateforme.
- Les chiffres (DD, PV, portées) sont des données du système de règles de chaque campagne (`rules-format.md`) : une règle donne la mécanique et la valeur par défaut, jamais une seconde copie.
- Un fait vit à un seul endroit ; les autres pages le citent par son nom, jamais par un numéro.
- Une décision tranchée en conversation devient une page dans le même commit.
- `bun run check:docs` vérifie les gabarits ci-dessous.

## Les trois gabarits

**Une page par règle, par fiche, par ticket. Elle tient sur un écran.**
Le titre de la page est le **Nom**. Chaque champ est un paragraphe à part
(une ligne vide entre deux champs). Schéma : 15 lignes au plus, dans un
bloc `text`. Listes : 5 puces au plus. Page : 45 lignes au plus.

### Règle — `regles/jeu/<domaine>/` ou `regles/campagne/<domaine>/`

````markdown
# Options verrouillées cachées

**État** : tranchée · 16/10/2026

**Règle** : une ou trois phrases, ce que vit le joueur.

**Schéma** :

```text
…
```

**Détails** :

- 5 puces au plus (champ facultatif)

**Exemple** : un cas joué, chiffres compris (champ facultatif).

**Pourquoi** : une ligne.

**Impacts** : campagne — … · plateforme — …

**Résolue par** : [Nom de la fiche](../../../plateforme/<fiche>.md) ou —
````

Une règle **à définir** remplace **Règle** par trois champs :
**Question**, **Options**, **Recommandation**.

### Fiche plateforme — `plateforme/`

````markdown
# Lecteur YouTube synchronisé

**État** : tranchée · avant la bible

**Résout** : [Nom de la règle](../regles/jeu/<domaine>/<règle>.md), ou la section de la bible pour une fiche de produit

**Solution** : une ou trois phrases (ou **Question** · **Options** · **Recommandation** tant qu'elle est à définir).

**Schéma** : (bloc text)

**Comment** : 5 puces au plus (facultatif)

**Limites** : ce qu'elle ne fait pas (facultatif).

**Pourquoi** : une ligne (facultatif).

**Tickets** : [`epic/verb-object`](../tickets/<epic>/<ticket>.md) ou —
````

### Ticket — `tickets/<épic>/`

````markdown
# play/talk-to-npcs · Parler aux PNJ

**État** : à faire · Jalon 1, phase « Parler »

**Réalise** : [Nom de la règle ou de la fiche](…)

**Objectif** : une phrase, ce que le joueur pourra faire.

**Schéma** : (bloc text, facultatif)

**Périmètre** : 5 puces au plus

**Fini quand** : vérifiable, en vraie soirée.

**Départ** : code ou tickets gelés repris, ou —
````

## Les états

```text
 règle, fiche   à définir ──▶ provisoire (à confirmer en partie) ──▶ tranchée · date
 ticket         à spécifier ──▶ à faire ──▶ en cours ──▶ (fini : archive/tickets/)
                                          └─▶ bloqué · raison
```

Une page qui change est réécrite sur place avec sa nouvelle date ; git
garde l'histoire.
