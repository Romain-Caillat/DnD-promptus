# Lecteur YouTube synchronisé

**État** : tranchée · avant la bible

**Résout** : [Bible, « Direction artistique et contenu »](../bible.md)

**Solution** : l'Auteur choisit plusieurs morceaux YouTube par scène ; le serveur annonce le morceau et l'instant de départ ; chaque joueur le joue sur son propre lecteur YouTube.

**Schéma** :

```text
 serveur : morceau + instant de départ ──▶ chaque joueur : lecteur YouTube visible
                                           démarre muet ──▶ « activer le son »
                                           pub ou décalage ──▶ se recale seul
```

**Comment** :

- Le lecteur reste visible (conditions d'utilisation de YouTube).
- Il démarre muet : chaque joueur active le son une fois.
- Une publicité peut décaler un joueur ; la lecture se recale d'elle-même.

**Limites** : un joueur peut entendre un morceau avec quelques secondes de décalage.

**Pourquoi** : choisie, la musique porte l'ambiance mieux qu'une génération, sans rien coûter à l'Auteur.

**Tickets** : —
