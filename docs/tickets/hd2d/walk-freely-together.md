# hd2d/walk-freely-together · Se promener librement ensemble

**État** : à faire · Jalon 1, phase « Le monde »

**Réalise** : [Bible, « Exploration »](../../bible.md)

**Objectif** : En exploration, on se promène sans cases, et on voit les autres joueurs.

**Périmètre** :

- déplacement continu au clavier, au clic ou à la manette ; collisions déduites des murs ; une seule vitesse
- positions validées par le serveur et diffusées aux joueurs du même lieu
- interaction à portée : une touche, une seule action, la chose la plus proche entourée d'or
- la grille réapparaît quand un combat commence sur place
- maquette : `docs/design/v2/exploration.html`

**Fini quand** : Quatre joueurs se promènent dans la taverne en même temps, sans traverser de mur, et chacun voit les autres bouger.

**Départ** : `characters/walk-in-four-directions`, `session/stream-live-changes`
