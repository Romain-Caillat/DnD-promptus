# Lecteur YouTube synchronisé

**État** : tranchée · avant la bible

**Résout** : [Musique choisie, synchronisée](../regles/jeu/apparence/musique-choisie-synchronisee.md)

**Solution** : le serveur annonce le morceau et l'instant de départ ; chaque joueur le joue sur son propre lecteur YouTube.

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

**Tickets** : —
