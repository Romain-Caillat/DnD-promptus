// Histoire de la campagne de démo « Le Donjon des gobelins », au format V2.
// Sert d'exemple de référence pour la génération par LLM (E2).

import type { CampaignStory, MapCell } from "../src/lib/engine/story";

/** Contour infranchissable d'un rectangle (murs, palissades). */
function walls(x0: number, y0: number, x1: number, y1: number, gaps: [number, number][] = []): MapCell[] {
  const cells: MapCell[] = [];
  const isGap = (x: number, y: number) => gaps.some(([gx, gy]) => gx === x && gy === y);
  for (let x = x0; x <= x1; x++) {
    for (let y = y0; y <= y1; y++) {
      const border = x === x0 || x === x1 || y === y0 || y === y1;
      if (border && !isGap(x, y)) cells.push({ x, y, terrain: "mur", blocked: true });
    }
  }
  return cells;
}

/** Fusionne des listes de cases : pour une même coordonnée, la dernière gagne. */
function merge(...lists: MapCell[][]): MapCell[] {
  const byKey = new Map<string, MapCell>();
  for (const c of lists.flat()) byKey.set(`${c.x},${c.y}`, c);
  return [...byKey.values()];
}

/** Construit l'histoire à partir des ids de fiches générés par le seed. */
export function buildDemoStory(id: (name: string) => string): CampaignStory {
  return {
    bible: {
      pitch:
        "Depuis trois semaines, des gobelins pillent Creux-d’Étain et ont enlevé Aldo, le fils du bourgmestre. Mais la bande de Grik n’est qu’un pion : sous la vieille chapelle, le Mort-Roi Aldric s’éveille et cherche un corps vivant.",
      tone: "Dark fantasy à hauteur de village : tension, superstition, pointes d’humour noir.",
      themes: ["la peur rend docile", "ce qu’on enterre finit par revenir", "le prix d’une promesse"],
      truths: [
        "Le roi Aldric a été enterré vivant sous la chapelle il y a deux siècles pour mettre fin à sa tyrannie.",
        "Les gobelins de Grik vénèrent le Mort-Roi par peur et lui apportent des offrandes.",
        "Aldo est vivant : le Mort-Roi le garde pour en faire son nouveau corps à la prochaine nouvelle lune.",
      ],
      secrets: [
        "Sœur Mira a ouvert le caveau il y a un mois en cherchant des reliques : c’est elle qui a réveillé le roi.",
        "Le vieux Tomas a forgé des clous de cercueil pour Grik contre la promesse que sa forge serait épargnée.",
      ],
      playerHook: "Le bourgmestre Bortrand offre 50 pièces d’or à qui ramènera son fils avant la nouvelle lune.",
      startSceneId: "sc_auberge",
    },
    fronts: [
      {
        id: "fr_gobelins",
        name: "La bande de Grik",
        goal: "Apporter assez d’offrandes au Mort-Roi pour s’attirer sa protection.",
        description: "Une douzaine de gobelins terrorisés et d’autant plus cruels.",
        steps: [
          { label: "Raids nocturnes", description: "Poulaillers vidés, portes forcées." },
          { label: "La grange brûle", description: "La grange des Morel part en fumée ; le village panique.", effects: [{ type: "display_text", text: "Une lueur orange monte au-dessus des toits : la grange des Morel brûle !" }] },
          { label: "Second enlèvement", description: "La petite Lise disparaît à son tour." },
          { label: "Assaut du village", description: "Grik attaque Creux-d’Étain en pleine nuit." },
        ],
      },
      {
        id: "fr_mortroi",
        name: "Le réveil du Mort-Roi",
        goal: "Reprendre un corps vivant et rebâtir son royaume.",
        description: "Une volonté ancienne, patiente, qui se sert de la peur des vivants.",
        steps: [
          { label: "Les morts remuent", description: "Grattements sous les dalles de la chapelle." },
          { label: "Squelettes dans le bois", description: "Des patrouilles d’os errent la nuit dans le Bois des Murmures." },
          { label: "Le rituel commence", description: "Aldo s’affaiblit ; ses yeux virent au gris." },
          { label: "Le roi possède Aldo", description: "Catastrophe : le Mort-Roi marche à nouveau parmi les vivants." },
        ],
      },
    ],
    scenes: [
      {
        id: "sc_auberge",
        title: "L’Auberge de l’If",
        summary: "Les personnages rencontrent le bourgmestre désespéré et entendent les rumeurs.",
        objective: "Accepter la mission et recueillir des pistes.",
        readAloud:
          "La pluie bat les volets de l’Auberge de l’If. Autour d’un feu qui ne réchauffe personne, les conversations s’arrêtent quand vous entrez. Au comptoir, un homme aux yeux rougis se lève : « Vous êtes des épées à louer ? Alors écoutez-moi. »",
        phase: "dialogue",
        locationEntityId: id("L’Auberge de l’If"),
        npcEntityIds: [id("Bortrand le Robuste"), id("Greta la Brasseuse")],
        monsterEntityIds: [],
        exits: [
          { toSceneId: "sc_village", label: "Interroger les villageois" },
          { toSceneId: "sc_bois", label: "Suivre la piste des pillards" },
        ],
        mapPlacement: { mapId: "map_vallee", x: 2, y: 4 },
        triggers: [],
        media: {
          imagePrompt: "taverne sombre, feu de cheminée, villageois inquiets, pluie aux fenêtres",
          musicQuery: "medieval tavern ambience rain",
        },
      },
      {
        id: "sc_village",
        title: "Creux-d’Étain sous la peur",
        summary: "Le village barricadé : forge, chapelle, maisons pillées.",
        objective: "Comprendre ce que veulent les gobelins et qui les aide.",
        readAloud:
          "Les rues pavées sont désertes. Des planches clouées à la hâte barrent les fenêtres. Seule la forge résonne encore, et plus loin, la cloche fêlée de la chapelle sonne sans raison.",
        phase: "exploration",
        locationEntityId: id("Village de Creux-d’Étain"),
        npcEntityIds: [id("Le vieux Tomas"), id("Sœur Mira")],
        monsterEntityIds: [],
        exits: [
          { toSceneId: "sc_chapelle", label: "Se rendre à la chapelle" },
          { toSceneId: "sc_bois", label: "Partir vers le bois" },
        ],
        mapPlacement: { mapId: "map_vallee", x: 2, y: 4 },
        triggers: [
          {
            id: "tg_grange",
            label: "La grange brûle",
            when: { frontStepAtLeast: { frontId: "fr_gobelins", step: 1 } },
            effects: [
              { type: "display_text", text: "Des cris : la grange des Morel est en flammes, des silhouettes vertes fuient vers le bois." },
              { type: "play_sound", soundId: "Porte qui grince" },
            ],
            oneShot: true,
          },
        ],
      },
      {
        id: "sc_bois",
        title: "Le Bois des Murmures",
        summary: "Une forêt trop silencieuse, des traces étranges, des loups affamés.",
        objective: "Trouver le camp des gobelins.",
        readAloud:
          "Sous les vieux chênes, le silence est total : pas un oiseau, pas un insecte. Dans la boue, des traces de petits pieds griffus… et d’autres, plus fines, qui ressemblent à des os.",
        phase: "travel",
        locationEntityId: id("Le Bois des Murmures"),
        npcEntityIds: [],
        monsterEntityIds: [id("Loup")],
        exits: [
          { toSceneId: "sc_camp", label: "Suivre les traces de gobelins" },
          { toSceneId: "sc_village", label: "Rentrer au village" },
        ],
        mapPlacement: { mapId: "map_vallee", x: 6, y: 2 },
        triggers: [],
        media: { musicQuery: "dark forest ambience night" },
      },
      {
        id: "sc_camp",
        title: "Le camp de Grik",
        summary: "Une clairière palissadée, une bannière, une douzaine de gobelins nerveux.",
        objective: "Vaincre ou faire parler les gobelins ; découvrir pour qui ils travaillent.",
        readAloud:
          "Derrière une palissade de pieux tordus, un feu crache une fumée âcre. Une bannière déchirée claque au vent : un gobelin grimaçant… coiffé d’une couronne d’os.",
        phase: "combat",
        npcEntityIds: [],
        monsterEntityIds: [id("Éclaireur gobelin"), id("Guerrier gobelin"), id("Chef gobelin")],
        exits: [{ toSceneId: "sc_chapelle", label: "Remonter vers la chapelle" }],
        mapPlacement: { mapId: "map_vallee", x: 7, y: 1 },
        battleMapId: "map_camp",
        triggers: [],
        media: { musicQuery: "goblin battle music" },
      },
      {
        id: "sc_chapelle",
        title: "La chapelle profanée",
        summary: "La chapelle de Sœur Mira, ses registres, et les dalles qui grattent la nuit.",
        objective: "Trouver l’entrée de la crypte et comprendre qui l’a ouverte.",
        readAloud:
          "La chapelle sent la cire froide et la terre remuée. Devant l’autel, une dalle n’est pas tout à fait à sa place. Sœur Mira serre son symbole sacré à s’en blanchir les doigts.",
        phase: "exploration",
        npcEntityIds: [id("Sœur Mira")],
        monsterEntityIds: [],
        exits: [{ toSceneId: "sc_crypte", label: "Descendre sous la dalle" }],
        mapPlacement: { mapId: "map_vallee", x: 3, y: 3 },
        triggers: [],
      },
      {
        id: "sc_crypte",
        title: "La Crypte oubliée",
        summary: "Couloirs humides, niches funéraires, morts qui se relèvent.",
        objective: "Traverser la crypte jusqu’au caveau royal.",
        readAloud:
          "L’escalier s’enfonce dans une obscurité qui sent la pierre mouillée. Le long des murs, des niches funéraires. Certaines sont vides. Quelque chose racle le sol, plus loin.",
        phase: "combat",
        locationEntityId: id("La Crypte oubliée"),
        npcEntityIds: [],
        monsterEntityIds: [id("Squelette"), id("Zombie")],
        exits: [{ toSceneId: "sc_trone", label: "Forcer la porte du caveau royal" }],
        mapPlacement: { mapId: "map_vallee", x: 3, y: 3 },
        battleMapId: "map_crypte",
        triggers: [
          {
            id: "tg_cri",
            label: "Un cri d’enfant",
            when: { any: [{ revelationKnown: "rv_aldo" }, { clueFound: "cl_mira" }] },
            effects: [{ type: "display_text", text: "Un appel faible résonne au fond de la crypte : « Papa ? »" }],
            oneShot: true,
          },
        ],
        media: { musicQuery: "dungeon ambience dripping water" },
      },
      {
        id: "sc_trone",
        title: "Le trône du Mort-Roi",
        summary: "Le caveau royal, Aldo enchaîné, le Revenant sur son trône.",
        objective: "Sauver Aldo et renvoyer le Mort-Roi au repos.",
        readAloud:
          "Le caveau est immense. Sur un trône de pierre noire, une silhouette couronnée se redresse dans un craquement d’os. À ses pieds, enchaîné, un garçon pâle lève vers vous des yeux gris.",
        phase: "combat",
        npcEntityIds: [],
        monsterEntityIds: [id("Revenant Mort-Roi")],
        exits: [],
        battleMapId: "map_crypte",
        triggers: [],
        media: { musicQuery: "epic boss battle orchestral dark" },
      },
    ],
    revelations: [
      { id: "rv_crypte", statement: "Les gobelins obéissent à quelque chose qui vit sous la chapelle.", importance: "critical" },
      { id: "rv_aldo", statement: "Aldo est vivant, retenu dans la crypte.", importance: "critical" },
      { id: "rv_mira", statement: "Sœur Mira a réveillé le Mort-Roi en ouvrant le caveau.", importance: "optional" },
    ],
    clues: [
      { id: "cl_banniere", revelationId: "rv_crypte", sceneId: "sc_camp", text: "La bannière de Grik porte une couronne d’os : le symbole gravé sur la porte de la crypte.", discovery: "Examiner la bannière du camp", check: { skill: "history", dc: 12 } },
      { id: "cl_tomas", revelationId: "rv_crypte", sceneId: "sc_village", text: "Tomas a forgé des clous de cercueil pour les gobelins, « pour leur roi d’en dessous ».", discovery: "Faire parler le vieux Tomas", check: { skill: "persuasion", dc: 13 } },
      { id: "cl_traces", revelationId: "rv_crypte", sceneId: "sc_bois", text: "Des empreintes d’os se mêlent aux traces de gobelins et mènent vers la chapelle.", discovery: "Étudier les traces dans la boue", check: { skill: "survival", dc: 12 } },
      { id: "cl_greta", revelationId: "rv_aldo", sceneId: "sc_auberge", text: "Greta a entendu un gobelin ivre parler d’un « garçon gardé pour le roi ».", discovery: "Gagner la confiance de Greta", check: { skill: "insight", dc: 10 } },
      { id: "cl_prisonnier", revelationId: "rv_aldo", sceneId: "sc_camp", text: "Un gobelin capturé avoue qu’on a « descendu le petit chez le roi ».", discovery: "Interroger un gobelin capturé", check: { skill: "intimidation", dc: 12 } },
      { id: "cl_mira", revelationId: "rv_aldo", sceneId: "sc_chapelle", text: "Mira entend chaque nuit une voix d’enfant sous les dalles.", discovery: "Rassurer Sœur Mira" },
      { id: "cl_registre", revelationId: "rv_mira", sceneId: "sc_chapelle", text: "Dernière ligne du registre, de la main de Mira : « Ouverture du caveau — reliques ».", discovery: "Consulter le registre de la chapelle", check: { skill: "investigation", dc: 11 } },
      { id: "cl_cle", revelationId: "rv_mira", sceneId: "sc_crypte", text: "La clé de bronze de la chapelle est encore dans la serrure du caveau.", discovery: "Examiner la porte du caveau" },
    ],
    maps: [
      {
        id: "map_monde",
        name: "La Marche d’Étain",
        level: "campaign",
        grid: { type: "hex", cols: 12, rows: 8 },
        backgroundPrompt: "carte fantasy dessinée à la main, vallée minière, forêts, montagnes au nord, parchemin",
        cells: [
          { x: 3, y: 4, label: "Vallée de Creux-d’Étain", childMapId: "map_vallee" },
          { x: 8, y: 2, label: "Château d’Aldric", terrain: "ruines" },
          ...[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11].map((x) => ({ x, y: 0, terrain: "montagne", blocked: true })),
          { x: 5, y: 5, terrain: "forêt" },
          { x: 6, y: 5, terrain: "forêt" },
          { x: 10, y: 6, label: "Port-Gris", terrain: "ville" },
        ],
      },
      {
        id: "map_vallee",
        name: "Vallée de Creux-d’Étain",
        level: "region",
        grid: { type: "hex", cols: 10, rows: 8 },
        backgroundPrompt: "vue de dessus d’une vallée, village minier, rivière, bois sombre au nord-est",
        cells: [
          { x: 2, y: 4, label: "Creux-d’Étain", terrain: "village" },
          { x: 3, y: 3, label: "Chapelle", terrain: "village", childMapId: "map_crypte" },
          { x: 6, y: 2, label: "Bois des Murmures", terrain: "forêt" },
          { x: 7, y: 1, label: "Camp de Grik", terrain: "forêt", childMapId: "map_camp" },
          { x: 5, y: 2, terrain: "forêt" },
          { x: 6, y: 1, terrain: "forêt" },
          { x: 7, y: 2, terrain: "forêt" },
          ...[0, 1, 2, 3, 4, 6, 7].map((y) => ({ x: 4, y, terrain: "rivière", blocked: true })),
          { x: 4, y: 5, label: "Gué", terrain: "rivière" },
        ],
      },
      {
        id: "map_camp",
        name: "Camp de Grik",
        level: "local",
        grid: { type: "square", cols: 16, rows: 12 },
        backgroundPrompt: "battlemap vue de dessus, clairière boueuse, palissade de pieux, feu de camp, tentes en peaux",
        cells: [
          ...walls(2, 2, 13, 9, [[7, 9], [8, 9]]),
          { x: 7, y: 5, label: "Feu de camp", terrain: "feu", blocked: true },
          { x: 11, y: 4, label: "Bannière de Grik" },
        ],
        tokens: [
          { entityId: id("Chef gobelin"), x: 11, y: 5 },
          { entityId: id("Guerrier gobelin"), x: 6, y: 7 },
          { entityId: id("Éclaireur gobelin"), x: 4, y: 4 },
        ],
      },
      {
        id: "map_crypte",
        name: "La Crypte oubliée",
        level: "local",
        grid: { type: "square", cols: 20, rows: 14 },
        backgroundPrompt: "battlemap vue de dessus, crypte de pierre, niches funéraires, flaques, caveau royal avec trône",
        cells: merge(
          walls(0, 0, 11, 13, [[11, 6], [5, 13]]),
          walls(11, 3, 19, 10, [[11, 6]]),
          [
            { x: 5, y: 12, label: "Escalier" },
            { x: 17, y: 6, label: "Trône du Mort-Roi", terrain: "trône", blocked: true },
            { x: 11, y: 6, label: "Porte du caveau" },
          ],
        ),
        tokens: [
          { entityId: id("Squelette"), x: 4, y: 5 },
          { entityId: id("Zombie"), x: 8, y: 3 },
          { entityId: id("Revenant Mort-Roi"), x: 16, y: 6 },
        ],
      },
    ],
  };
}
