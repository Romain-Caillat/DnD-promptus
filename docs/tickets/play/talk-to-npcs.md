# play/talk-to-npcs · Parler aux PNJ

**État** : à faire · Jalon 1, phase « Parler »

**Réalise** : [Options verrouillées cachées](../../regles/jeu/dialogue/options-verrouillees-cachees.md) · [Option bloquée par l'argent](../../regles/jeu/dialogue/option-bloquee-par-l-argent.md) · [Rejoindre une conversation](../../regles/jeu/dialogue/rejoindre-une-conversation.md)

**Objectif** : Un joueur parle à un PNJ en choisissant ses répliques, et un autre peut le rejoindre.

**Schéma** :

```text
 Yann ──▶ Gorvin ◀── Léa rejoint, souffle son option ──▶ Yann choisit
```

**Périmètre** :

- portrait, texte du narrateur, choix, indice de Perspicacité
- jets avec le dé en 3D (`hd2d/roll-dice-in-3d`)
- options fermées cachées ; option trop chère grisée avec ce qui manque
- rejoindre : celui qui a lancé choisit, l'autre ne souffle que ce que lui seul débloque
- maquette : `docs/design/v2/dialogue.html`

**Fini quand** : À la soirée, deux joueurs mènent une conversation ensemble.

**Départ** : `player/play-scene` (gelé), `ui/roll-faceted-dice`
