# Carte des docs

```
 bible.md              la vision : le jeu qu'on fait, ses inspirations
 regles/jeu/           CE QUI DOIT ÊTRE VRAI pour le joueur
 regles/campagne/      CE QUI DOIT ÊTRE VRAI pour l'Auteur, et ce qu'il règle
          │ chaque règle est « résolue par » ▼
 plateforme/           COMMENT on la rend vraie (moteur, données, temps réel, IA)
          │ chaque fiche est « réalisée par » ▼
 TICKETS.md            le travail
```

- **Une règle sans « résolue par » est un trou de plateforme.** Les index
  les montrent : c'est ce qui reste à définir.
- **Une limite technique ne change jamais une règle en douce** : elle
  remonte comme une nouvelle règle à trancher.
- **Référence par défaut** : quand une règle est muette, on fait comme la
  référence de son domaine (voir l'index de `regles/jeu/`). On ne décide
  que des écarts.
- Un fait vit à un seul endroit ; les autres docs le citent par son nom.
- Une décision tranchée en conversation devient un bloc dans le même commit.

Les docs techniques déjà écrites (formats YAML, installation, MCP…)
restent à leur place : du code et des migrations les citent. L'index de
`plateforme/` les recense.

## Le squelette d'un bloc

Un fichier par domaine (`dialogue.md`, `groupe.md`…), un bloc par règle.
**Tous les blocs ont la même forme**, qu'ils soient tranchés ou à définir :

````
## <Nom de la règle>
> **<état>** <date> · résolue par : <fiche plateforme | —>

```
<schéma, 15 lignes au plus>
```

<tranchée ou provisoire>   au plus 5 puces de règle
<à définir>                **Question** · **Options** · **Recommandation**

**Pourquoi** : une ligne.
**↳ campagne** : … · **↳ plateforme** : …   (ou ↳ jeu, selon le dossier)
````

- **États** : `à définir` · `provisoire` (valeur par défaut, à confirmer
  en partie de test) · `tranchée`.
- Une fiche de `plateforme/` a le même squelette, avec **résout :**
  (les règles qu'elle sert) à la place de « résolue par ».
- Une règle qui change est réécrite sur place avec sa nouvelle date ;
  git garde l'histoire. Son ancienne forme ne reste nulle part ailleurs.
- On nomme toujours un bloc par son titre, jamais par un numéro.
