# Écran de combat

**État** : tranchée · 11/10/2026

**Résout** : [L'ordre des tours](../regles/jeu/combat/l-ordre-des-tours.md) · [Ton tour](../regles/jeu/combat/ton-tour.md) · [Attaquer](../regles/jeu/combat/attaquer.md) · [Lire les intentions ennemies](../regles/jeu/combat/lire-les-intentions-ennemies.md)

**Solution** : Le combat se joue dans le décor du lieu : la caméra monte en vue tactique pour choisir, plonge en HD-2D pour le coup, et une banderole marque les moments rares.

**Schéma** :

```text
 1. TACTIQUE (vue de dessus) ──▶ 2. COUP (HD-2D) ──▶ 3. BANDEROLE (rare)
    même scène, caméra haute       la caméra plonge     critique, sort ultime
    grille, portées, choix         sur l'action         portrait « en action »
```

**Comment** :

- frise d'initiative ; Action, Action bonus, Déplacement toujours visibles
- aperçu avant de valider : % de toucher et dégâts ; minuteur du tour
- intention lue affichée au-dessus de l'ennemi
- animations des ennemis accélérées ou désactivables ; la banderole toujours jouée

**Limites** : comment la grille se pose sur un décor peint reste à dessiner.

**Pourquoi** : les règles de DnD avec la lisibilité de Fire Emblem et le décor de Triangle Strategy.

**Tickets** : [`design/draw-combat-planes`](../tickets/design/draw-combat-planes.md) · [`hd2d/switch-combat-planes`](../tickets/hd2d/switch-combat-planes.md) · [`hd2d/stage-combat-actions`](../tickets/hd2d/stage-combat-actions.md) · [`hd2d/play-critical-banner`](../tickets/hd2d/play-critical-banner.md) · [`play/fight-turn`](../tickets/play/fight-turn.md)
