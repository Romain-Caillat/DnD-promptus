# Écran d'exploration

**État** : tranchée · 12/10/2026

**Résout** : [Explorer librement](../regles/jeu/exploration/explorer-librement.md) · [Perception passive](../regles/jeu/exploration/perception-passive.md)

**Solution** : Chaque joueur voit le lieu en HD-2D, les autres joueurs présents, et seulement les indicateurs qu'il a le droit de voir.

**Schéma** :

```text
 [!] quête   [?] mini-jeu   ◆ doré : découvert par ta Perception
 la chose la plus proche est entourée d'or ──▶ [E] une seule action proposée
```

**Comment** :

- déplacement en ZQSD (WASD en option) ou à la manette
- une découverte par Perception est annoncée une fois, en privé
- maquette : `docs/design/v2/exploration.html`

**Tickets** : [`hd2d/walk-freely-together`](../tickets/hd2d/walk-freely-together.md)
