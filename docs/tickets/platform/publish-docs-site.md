# platform/publish-docs-site · Lire la doc depuis la plateforme

**État** : en cours · hors jalon

**Réalise** : [Carte des docs](../../README.md)

**Objectif** : Romain lit les règles, les fiches et les tickets dans un site clair servi par le serveur de jeu, sous `/docs/`.

**Schéma** :

```text
 docs/**/*.md ──▶ bun run check:docs ──▶ MkDocs Material ──▶ site/
 serveur :  /        le jeu
            /docs/   la doc (règles, plateforme, tickets)
```

**Périmètre** :

- une page par règle, par fiche, par ticket, aux gabarits de la carte des docs
- un contrôle des gabarits lancé par `bun run lint`
- le site MkDocs, construit dans l'image Docker et servi sous `/docs/`
- `bun run docs` pour le lire en local
- hypothèse : installation, sauvegarde et accès MCP restent hors du site, qui est joignable depuis Internet

**Fini quand** : Romain ouvre `/docs/` sur le serveur et y relit une règle, sa fiche et son ticket.

**Départ** : —
