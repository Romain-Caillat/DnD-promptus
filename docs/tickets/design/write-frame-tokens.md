# design/write-frame-tokens · Porter le cadre Octopath dans le code

**État** : à faire · Jalon 1, phase « Maquettes et règles »

**Réalise** : [Interface façon Octopath](../../regles/jeu/apparence/interface-facon-octopath.md)

**Objectif** : Le cadre validé vit dans le code, pas seulement dans une maquette.

**Périmètre** :

- porter dans `front/src/styles/tokens.css` l'habillage B de `docs/design/v2/cadre-pc.html`
- panneaux, double filet doré, boutons, touches clavier et manette, curseur de focus, notifications
- les zones du cadre : le panneau MJ décale le jeu
- la page `/reference` les montre
- les couleurs et les formes des stats ne changent pas

**Fini quand** : La page `/reference` montre le cadre comme la maquette, et Romain l'a relue.

**Départ** : `ui/write-design-tokens`, `ui/adopt-pixel-menu` (gelés)
