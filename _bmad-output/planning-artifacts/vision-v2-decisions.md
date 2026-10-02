---
document: Vision V2 — Décisions produit
product: Promptus
date: 2026-10-02
status: Validé (base de travail)
supersedes:
  - product-brief-promptus.md (règles de design et anti-features IA)
  - mvp-retrospective-and-pivot.md (plan de reprise)
---

# Promptus V2 — Décisions produit

## Constat sur le V1

Les features sont marquées « done » mais ne sont pas utilisables : beaucoup
sont mockées (boutons « not yet wired », effets narratifs sans effet,
`scene_markers` jamais lus) ou non connectées entre elles (révélations
invisibles faute d'interface joueur, participants figés au lancement, pas de
fin de session). Le V1 est un simulateur de combat, pas un jeu.

## Décisions

| Sujet | Décision |
|---|---|
| Philosophie | L'app planifie et génère la campagne (LLM, image, vidéo, musique) |
| Création | Seul le MJ construit le jeu. Les joueurs ne créent pas de contenu |
| MJ | Hybride : MJ humain + co-MJ LLM. Le LLM propose, l'humain valide |
| Mode de jeu prioritaire | **Tout le monde à distance** (le mode table + TV viendra ensuite) |
| Cartes | Toutes avec grille, 3 niveaux : campagne → région → combat/lieu |
| Règles | Claires et visibles pour les joueurs, une règle de déplacement par échelle |
| Budget génération | Réglable par le MJ |
| Langue | Application et contenus en français |

## Rôles

- **MJ** : génère, relit et édite la campagne en préparation. En direct, il
  pilote la session, valide les propositions du co-MJ et révèle les contenus.
- **Co-MJ (LLM)** : en préparation, il génère la structure et le contenu. En
  direct, il propose la narration, les répliques des PNJ, les conséquences et
  le nœud suivant. Il ne montre rien aux joueurs sans validation du MJ.
- **Joueurs** : à distance, chacun sur sa propre interface (fiche, carte,
  actions possibles, médias révélés).

## Architecture d'histoire (lisible par un LLM)

Graphe de nœuds relié par des indices, avec des menaces qui progressent.

```
Campagne (bible) : pitch, ton, vérités du monde, secrets, style visuel/sonore
├── Fronts : menaces qui avancent seules (horloge en 4-6 étapes)
├── Nœuds (scènes / lieux / situations)
│     objectif · texte à lire · PNJ présents · indices · sorties
│     déclencheurs · médias · case de carte rattachée
├── Entités : PNJ (motivation, secret, voix, relations), lieux, factions, objets
├── Indices : chaque révélation clé est accessible par ≥ 3 indices
│     placés dans des nœuds différents
└── État vivant : état du monde + journal + résumés par session
```

Principes :
- Des identifiants stables et des blocs courts : le contexte LLM se limite à
  la bible, au nœud courant, aux entités liées et au résumé.
- Un graphe plutôt qu'un scénario linéaire, pour absorber l'improvisation.
- Les fronts donnent au LLM « ce qui se passe si personne n'agit ».
- Les déclencheurs réutilisent les effets narratifs du moteur
  (`set_state`, `trigger_event`, `reveal_entity`, `set_relation`…).

## Cartes

| Niveau | Grille | 1 case = | Temps | Déplacement |
|---|---|---|---|---|
| Campagne | hexagones | ~10 km | jour | X cases/jour selon l'allure, 1 jet de rencontre/jour |
| Région | hexagones | ~500 m | heure | exploration, découverte des lieux |
| Combat / lieu | carrés | 1,5 m | tour (6 s) | vitesse 9 m = 6 cases |

- Une case peut ouvrir une carte de niveau inférieur. Chaque nœud du graphe
  est rattaché à une case.
- Le MJ contrôle le brouillard de guerre.
- **Les données et le décor sont séparés.** La grille, les murs, les
  obstacles et les positions sont des données, seule référence pour les
  règles. L'image générée n'est qu'un fond, et l'app dessine la grille
  par-dessus.
- L'interface joueur surligne les cases atteignables et les portées.

## Pipeline de génération (préparation)

1. Le MJ saisit un pitch : ton, durée, niveau, nombre de joueurs, style.
2. Le LLM produit la structure (bible, fronts, nœuds, entités, indices,
   cartes en données), validée par un schéma.
3. Le MJ relit et édite.
4. Les médias sont générés en jobs asynchrones : images (PNJ, lieux,
   scènes, fonds de carte), musique et ambiances, vidéos courtes d'intro de
   scène. Tout est pré-généré et mis en cache, rien n'est généré en direct
   par défaut.
5. Le MJ fixe le budget. L'app estime le coût avant chaque lot de génération.

Les fournisseurs (LLM, image, vidéo, musique) restent derrière des
interfaces interchangeables, comme le fait déjà `lib/ai/image-generator.ts`.

## Jeu en direct à distance

- Le MJ crée une session et partage un lien d'invitation. Les joueurs
  rejoignent sans compte : un pseudo plus le choix de leur personnage.
- La synchronisation est en temps réel (fini le rafraîchissement toutes les
  5 s).
- Écran MJ : nœud courant, propositions du co-MJ, carte complète, PNJ, état.
- Écran joueur : carte (zone révélée), fiche, actions possibles, médias
  révélés, journal.
- Voix : on passe par un outil externe (Discord…) au départ. Hypothèse à
  valider.

## Ce qu'on garde du V1

- Le moteur de règles (`lib/engine`) : effets, dés, conditions, résolveur
  d'attaque.
- Le schéma d'entités, la validation Zod, Drizzle et Postgres.
- L'abstraction de génération d'images et le guide de style.
- Le catalogue audio.

Le poste de pilotage V1 (barre d'actions par phase) est abandonné au profit
des écrans MJ et joueur.

## Règles de design V2

1. Le MJ a le dernier mot : l'IA propose, le MJ valide.
2. L'IA travaille en préparation **et** en direct, mais rien n'atteint les
   joueurs sans validation.
3. Les règles sont claires : le joueur voit toujours ce qu'il peut faire.
4. Pas de mock : une feature n'est « done » que si elle est utilisable de
   bout en bout dans une vraie session.
5. Français d'abord.

## Découpage proposé (epics)

| # | Epic | Contenu |
|---|---|---|
| E0 | Assainissement V1 | Tests (séparer l'intégration), lint, `.env.example`, suppression des mocks, passage en français |
| E1 | Modèle de campagne | Schéma : bible, fronts, nœuds, indices, cartes, état vivant |
| E2 | Génération de campagne | Pitch → structure par LLM → édition par le MJ |
| E3 | Pipeline média | Jobs async image / musique / vidéo, budget, cache |
| E4 | Cartes à grille | 3 niveaux, données + fond, brouillard, navigation entre niveaux |
| E5 | Session temps réel | Lobby, lien d'invitation, synchronisation |
| E6 | Interface joueur | Fiche, carte, actions possibles, médias, journal |
| E7 | Cockpit MJ + co-MJ | Nœud courant, propositions LLM, validation, révélations |
| E8 | Combat sur grille | Déplacement, portées, initiative, réutilisation du résolveur |
| E9 | Continuité | Journal, résumés de session, récap |

**Tranche jouable visée :** une mini-campagne générée (3-5 nœuds, 1 carte
de chaque niveau), jouée à distance de bout en bout avec un combat sur grille.

## Questions ouvertes

- Le temps réel : WebSocket (serveur dédié) ou SSE + POST dans Next.js ?
- Les fournisseurs image, vidéo et musique à brancher en premier.
- Le système de règles : on garde la 5e (SRD), ou on la simplifie ?
