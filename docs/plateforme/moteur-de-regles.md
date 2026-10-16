# Moteur de règles en données

**État** : tranchée · avant la bible

**Résout** : [Créer ton personnage](../regles/jeu/personnage/creer-ton-personnage.md) · [Les six stats](../regles/jeu/personnage/les-six-stats.md) · [Faire un jet](../regles/jeu/jets/faire-un-jet.md) · [L'ordre des tours](../regles/jeu/combat/l-ordre-des-tours.md) · [Ton tour](../regles/jeu/combat/ton-tour.md) · [Attaquer](../regles/jeu/combat/attaquer.md) · [Tomber à 0 PV](../regles/jeu/combat/tomber-a-0-pv.md)

**Solution** : Chaque campagne a son système de règles, un fichier de données ; le moteur du serveur l'applique et reste seul juge de chaque jet, déplacement et attaque.

**Schéma** :

```text
 content/rules/<campagne>/vN.yaml ──▶ moteur (serveur, seul juge) ──▶ jets, attaques, PV
                                  └──▶ page de règles et fiche du joueur
```

**Comment** :

- format : `docs/rules-format.md`
- le préréglage SRD donne les valeurs par défaut citées dans les règles
- contrôle et combats simulés avant la table (`bun run rules-report`)

**Limites** : ni réaction ni attaque d'opportunité ne sont modélisées.

**Pourquoi** : les règles d'une campagne se changent sans toucher au code, et le joueur les lit telles qu'elles s'appliquent.

**Tickets** : [`engine/rewrite-corsaires-rules`](../tickets/engine/rewrite-corsaires-rules.md) · [`engine/fix-stats-by-class`](../tickets/engine/fix-stats-by-class.md) · [`engine/time-turns`](../tickets/engine/time-turns.md)
