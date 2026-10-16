# engine/rewrite-corsaires-rules · Réécrire les règles des Corsaires près de la 5e

**État** : à faire · Jalon 1, phase « Maquettes et règles »

**Réalise** : [Moteur de règles en données](../../plateforme/moteur-de-regles.md)

**Objectif** : Les joueurs retrouvent l'esprit DnD qu'ils connaissent, sans la complexité de la 5e.

**Périmètre** :

- réécrire `content/rules/corsaires/` depuis le préréglage SRD (`content/rules/srd/`)
- six caractéristiques et modificateurs 5e, compétences, maîtrise, CA, avantage et désavantage
- Action / Action bonus / Réaction / Déplacement, jets contre la mort
- classes et ambiance des Corsaires gardées, peu de sorts et d'aptitudes jusqu'au niveau 5
- les défauts connus de la V1 disparaissent ; ce qui n'est pas modélisé est écrit (`NOT MODELLED`)

**Fini quand** : `engine/lint-rule-system` ne signale plus rien de bloquant, la simulation de la bagarre du quai est équilibrée, et Romain a relu le fichier.

**Départ** : `engine/add-srd-preset`, `engine/simulate-fights`
