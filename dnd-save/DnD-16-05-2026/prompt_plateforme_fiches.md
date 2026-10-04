# Prompt — Plateforme de Fiches de Personnages DnD

## Objectif

Construire une application web (serveur) pour gérer les fiches de personnages du one-shot "Corsaires de la Couronne". Le MJ crée une session, donne un code aux joueurs, chaque joueur rejoint et remplit sa fiche (classe + identité). Tout est persisté en DB. Le MJ peut éditer les fiches en temps réel pendant la partie.

---

## Architecture générale

### Stack suggérée
- **Backend** : Python (Flask/FastAPI) ou Node.js (Express/Next.js)
- **DB** : SQLite (simple, un fichier) ou PostgreSQL
- **Frontend** : HTML/CSS/JS vanilla (reproduire le style existant) ou React
- **Temps réel** : WebSocket ou polling pour les updates MJ ↔ joueurs

### Modèles de données

```
Session {
  id: UUID
  code: string (6 chars, ex: "PIRATE")  // code que les joueurs utilisent pour rejoindre
  name: string ("Corsaires de la Couronne")
  template: string ("corsaires")
  created_at: datetime
  status: enum("lobby", "in_game", "finished")
  dm_id: UUID (ref User)
}

User {
  id: UUID
  name: string  // nom du joueur (Antoine, Justin, etc.)
  role: enum("dm", "player")
  session_id: UUID (ref Session)
}

CharacterSheet {
  id: UUID
  user_id: UUID (ref User)
  session_id: UUID (ref Session)
  version: int (auto-increment à chaque save)
  
  // Classe
  class_id: string (bretteur, canonnier, navigateur, chirurgien, quartier-maitre, vigie, flibustier, boucanier)
  
  // Identité
  char_name: string
  char_region: string
  char_description: text
  char_background: text
  
  // Combat
  hp: int (default 10)
  hp_max: int (default 10)
  xp: int (default 0)
  xp_max: int (default 5)
  upgrade_points: int (default 0)
  player_level: int (default 1)
  total_xp_gained: int (default 0)
  
  // Stats (scores)
  stat_for: int
  stat_dex: int
  stat_con: int
  stat_int: int
  stat_sag: int
  stat_cha: int
  
  // Inventaire
  gold: int (default 10)
  inventory: JSON  // [{name, desc, qty, starter}]
  
  // Attaque libre
  free_slot_name: string
  free_slot_desc: string
  free_slot_tags: string
  
  // Notes
  session_notes: text
  
  updated_at: datetime
}
```

---

## Flux utilisateur

### 1. MJ — Créer une session
- Le MJ se connecte (login simple, username + password ou juste un pseudo protégé)
- Il crée une session → choisit le template "Corsaires de la Couronne"
- Le système génère un **code de session** (ex: `PIRATE`, `CORSAIR`, 6 chars)
- Le MJ voit un **lobby** avec la liste des joueurs connectés (en temps réel)

### 2. Joueur — Rejoindre
- Le joueur arrive sur la page d'accueil
- Il entre le **code de session** + son **prénom** (Antoine, Justin, Nadia, Nicolas, Paul, Pierre)
- Il arrive sur sa **fiche de personnage vierge**
- Il peut choisir sa **classe** et remplir son **identité** (nom perso, région, description, background)
- Il ne touche PAS aux stats (elles sont déterminées par la classe)
- Il clique "Valider" → la fiche est sauvegardée en DB

### 3. MJ — Dashboard en jeu
- Le MJ voit **toutes les fiches** des joueurs (lecture + écriture)
- Il peut **modifier HP, XP, inventaire, notes** de n'importe quel joueur
- Les modifications sont **poussées en temps réel** sur la fiche du joueur (WebSocket)
- Le MJ a un bouton "Donner +1 XP" / "Infliger X dégâts" / "Ajouter objet" rapide

### 4. Joueur — Pendant la partie
- La fiche se met à jour en temps réel quand le MJ modifie quelque chose
- Le joueur peut voir ses stats, HP, attaques disponibles, inventaire
- Le joueur peut éditer : notes de session, slot d'attaque libre
- Le joueur ne peut PAS modifier ses stats/HP/XP (seul le MJ le fait)

---

## Template "Corsaires de la Couronne"

### Données des classes

```javascript
const CLASSES = {
  bretteur: {
    name: "Bretteur",
    desc: "Combattant agile au sabre, maître du duel et de l'esquive.",
    stats: ["for", "dex"],  // stats principales (highlighted)
    baseStats: { for: 13, dex: 14, con: 10, int: 8, sag: 10, cha: 9 },
    ca: 12,  // 10 + mod DEX(14) = 10 + 2
    items: [
      { name: "Sabre d'abordage", desc: "Lame courbe idéale pour le combat rapproché en mer", qty: 1 },
      { name: "Dague de ceinture", desc: "Arme secondaire discrète, utile en dernier recours", qty: 1 }
    ],
    attacks: [
      {
        name: "Estocade",
        lvl: 1,
        desc: "Coup rapide et précis au point faible de l'adversaire.",
        tags: [
          { type: "dmg", value: "3 dégâts" },
          { type: "prec", value: "+2 précision" },
          { type: "cd", value: "Pas de cooldown" }
        ]
      },
      {
        name: "Riposte en quarte",
        lvl: 3,
        desc: "Pare l'attaque suivante et contre-attaque immédiatement.",
        tags: [
          { type: "dmg", value: "2 dégâts (contre)" },
          { type: "buff", value: "-2 dégâts reçus ce tour" },
          { type: "cd", value: "CD: 2 tours" }
        ]
      },
      {
        name: "Danse des lames",
        lvl: 7,
        desc: "Enchaînement dévastateur sur tous les ennemis proches.",
        tags: [
          { type: "dmg", value: "4 dégâts (AoE mêlée)" },
          { type: "prec", value: "+1 précision" },
          { type: "ctrl", value: "-1 précision ennemis (1 tour)" },
          { type: "cd", value: "CD: 4 tours" }
        ]
      }
    ]
  },

  canonnier: {
    name: "Canonnier",
    desc: "Expert en artillerie et explosifs, terreur des navires ennemis.",
    stats: ["int", "for"],
    baseStats: { for: 12, dex: 10, con: 10, int: 14, sag: 9, cha: 8 },
    ca: 10,
    items: [
      { name: "Pistolet à silex", desc: "Arme à feu à un coup, puissante mais lente à recharger", qty: 1 },
      { name: "Sacoche de poudre", desc: "Poudre noire pour recharger ou fabriquer des explosifs (3 charges)", qty: 3 }
    ],
    attacks: [
      {
        name: "Tir de silex",
        lvl: 1,
        desc: "Tir de pistolet à distance. Puissant mais imprécis.",
        tags: [
          { type: "dmg", value: "4 dégâts" },
          { type: "prec", value: "+0 précision" },
          { type: "cd", value: "CD: 1 tour" }
        ]
      },
      {
        name: "Bombe à mèche",
        lvl: 3,
        desc: "Lance un explosif artisanal. Touche une zone.",
        tags: [
          { type: "dmg", value: "3 dégâts (AoE)" },
          { type: "ctrl", value: "Stun 1 tour (jet SAG)" },
          { type: "cd", value: "CD: 3 tours" }
        ]
      },
      {
        name: "Salve de bordée",
        lvl: 7,
        desc: "Volée de canons dévastatrice. Nécessite navire ou position.",
        tags: [
          { type: "dmg", value: "7 dégâts (ligne)" },
          { type: "prec", value: "+2 précision" },
          { type: "ctrl", value: "Cibles renversées (perdent 1 tour)" },
          { type: "cd", value: "CD: 5 tours" }
        ]
      }
    ]
  },

  navigateur: {
    name: "Navigateur",
    desc: "Maître des cartes et des étoiles, guide infaillible des océans.",
    stats: ["sag", "int"],
    baseStats: { for: 8, dex: 10, con: 9, int: 13, sag: 14, cha: 10 },
    ca: 10,
    items: [
      { name: "Compas enchanté", desc: "Pointe toujours vers le nord véritable, même en tempête", qty: 1 },
      { name: "Carte des courants", desc: "Carte détaillée révélant les courants marins secrets", qty: 1 }
    ],
    attacks: [
      {
        name: "Lecture des vents",
        lvl: 1,
        desc: "Prédit les mouvements et guide un allié.",
        tags: [
          { type: "buff", value: "+2 précision à un allié (1 tour)" },
          { type: "cd", value: "Pas de cooldown" }
        ]
      },
      {
        name: "Brume marine",
        lvl: 3,
        desc: "Brouillard épais qui désoriente les ennemis.",
        tags: [
          { type: "ctrl", value: "-2 précision ennemis (2 tours)" },
          { type: "buff", value: "Alliés insaisissables" },
          { type: "cd", value: "CD: 3 tours" }
        ]
      },
      {
        name: "Appel du Maelström",
        lvl: 7,
        desc: "Déchaîne les courants. Piège mortel en zone.",
        tags: [
          { type: "dmg", value: "3 dégâts/tour (zone, 3 tours)" },
          { type: "ctrl", value: "Mouvement réduit de moitié" },
          { type: "cd", value: "CD: 5 tours" }
        ]
      }
    ]
  },

  chirurgien: {
    name: "Chirurgien de bord",
    desc: "Soigneur et empoisonneur, il décide qui vit et qui meurt.",
    stats: ["sag", "con"],
    baseStats: { for: 8, dex: 10, con: 13, int: 10, sag: 14, cha: 9 },
    ca: 10,
    items: [
      { name: "Sacoche médicale", desc: "Outils chirurgicaux, bandages et herbes médicinales", qty: 1 },
      { name: "Fiole de rhum fortifiant", desc: "Restaure 3 HP par utilisation", qty: 2 }
    ],
    attacks: [
      {
        name: "Premiers soins",
        lvl: 1,
        desc: "Pansement rapide en plein combat.",
        tags: [
          { type: "heal", value: "+3 HP (allié ou soi)" },
          { type: "cd", value: "Pas de cooldown" }
        ]
      },
      {
        name: "Lame empoisonnée",
        lvl: 3,
        desc: "Enduit une arme alliée de poison paralysant.",
        tags: [
          { type: "dmg", value: "+2 dégâts poison (2 tours)" },
          { type: "ctrl", value: "Cible -1 à tous ses jets" },
          { type: "cd", value: "CD: 3 tours" }
        ]
      },
      {
        name: "Chirurgie de guerre",
        lvl: 7,
        desc: "Opération miraculeuse. Relève ou soigne massivement.",
        tags: [
          { type: "heal", value: "+7 HP ou relève un allié à 0" },
          { type: "buff", value: "+1 toutes stats (1 tour)" },
          { type: "cd", value: "CD: 5 tours" }
        ]
      }
    ]
  },

  "quartier-maitre": {
    name: "Quartier-maître",
    desc: "Négociateur et gestionnaire, la voix de la raison (ou de la ruse).",
    stats: ["cha", "int"],
    baseStats: { for: 8, dex: 9, con: 10, int: 13, sag: 10, cha: 14 },
    ca: 9,
    items: [
      { name: "Registre de comptes", desc: "Contient les dettes et faveurs de nombreux contacts", qty: 1 },
      { name: "Lettre de marque (fausse)", desc: "Document officiel contrefait, utile pour bluffer", qty: 1 }
    ],
    attacks: [
      {
        name: "Ordre galvanisant",
        lvl: 1,
        desc: "Cri de ralliement qui motive un allié.",
        tags: [
          { type: "buff", value: "+1 dégâts et +1 précision (allié, 1 tour)" },
          { type: "cd", value: "Pas de cooldown" }
        ]
      },
      {
        name: "Intimidation",
        lvl: 3,
        desc: "Démoralise un ennemi. Jet CHA vs SAG.",
        tags: [
          { type: "ctrl", value: "Cible apeurée (fuit ou passe son tour)" },
          { type: "cd", value: "CD: 2 tours" }
        ]
      },
      {
        name: "Discours du capitaine",
        lvl: 7,
        desc: "Discours épique qui transcende l'équipage.",
        tags: [
          { type: "buff", value: "TOUS alliés +2 dégâts +2 précision (2 tours)" },
          { type: "heal", value: "+2 HP à tous les alliés" },
          { type: "cd", value: "CD: 5 tours" }
        ]
      }
    ]
  },

  vigie: {
    name: "Vigie",
    desc: "Éclaireur aux yeux perçants, rien n'échappe à son regard.",
    stats: ["dex", "sag"],
    baseStats: { for: 9, dex: 14, con: 10, int: 10, sag: 13, cha: 8 },
    ca: 12,
    items: [
      { name: "Longue-vue en laiton", desc: "Permet de repérer navires et dangers à grande distance", qty: 1 },
      { name: "Grappin et corde", desc: "30m de corde solide avec grappin, pour escalader ou aborder", qty: 1 }
    ],
    attacks: [
      {
        name: "Tir embusqué",
        lvl: 1,
        desc: "Tir précis depuis une position surélevée.",
        tags: [
          { type: "dmg", value: "3 dégâts" },
          { type: "prec", value: "+3 précision (+4 si en hauteur)" },
          { type: "cd", value: "Pas de cooldown" }
        ]
      },
      {
        name: "Point faible repéré",
        lvl: 3,
        desc: "Désigne une cible. Prochain allié à l'attaquer a un bonus.",
        tags: [
          { type: "buff", value: "Prochain allié: +3 précision +2 dégâts" },
          { type: "cd", value: "CD: 2 tours" }
        ]
      },
      {
        name: "Œil du faucon",
        lvl: 7,
        desc: "Concentration absolue. Tir garanti au point vital.",
        tags: [
          { type: "dmg", value: "6 dégâts (auto-touche, ignore armure)" },
          { type: "ctrl", value: "Saignement -1 HP/tour (3 tours)" },
          { type: "cd", value: "CD: 4 tours" }
        ]
      }
    ]
  },

  flibustier: {
    name: "Flibustier",
    desc: "Brute d'abordage, premier à sauter sur le pont ennemi.",
    stats: ["for", "con"],
    baseStats: { for: 14, dex: 9, con: 13, int: 8, sag: 10, cha: 10 },
    ca: 9,
    items: [
      { name: "Hache d'abordage", desc: "Lourde hache dévastatrice au corps à corps", qty: 1 },
      { name: "Bouclier de tonneau", desc: "Couvercle de tonneau renforcé, protection improvisée", qty: 1 }
    ],
    attacks: [
      {
        name: "Coup de hache",
        lvl: 1,
        desc: "Frappe lourde et brutale.",
        tags: [
          { type: "dmg", value: "4 dégâts" },
          { type: "prec", value: "+1 précision" },
          { type: "cd", value: "Pas de cooldown" }
        ]
      },
      {
        name: "Cri de guerre",
        lvl: 3,
        desc: "Hurlement terrifiant qui protège et effraie.",
        tags: [
          { type: "buff", value: "-2 dégâts reçus (2 tours, soi)" },
          { type: "ctrl", value: "Ennemis proches apeurés (1 tour)" },
          { type: "cd", value: "CD: 3 tours" }
        ]
      },
      {
        name: "Charge dévastatrice",
        lvl: 7,
        desc: "Se jette avec la force d'un boulet de canon.",
        tags: [
          { type: "dmg", value: "6 dégâts + renverse" },
          { type: "ctrl", value: "Cible perd son prochain tour" },
          { type: "buff", value: "Ignore dégâts reçus ce tour" },
          { type: "cd", value: "CD: 4 tours" }
        ]
      }
    ]
  },

  boucanier: {
    name: "Boucanier",
    desc: "Chasseur furtif et trappeur, patient et mortel.",
    stats: ["dex", "con"],
    baseStats: { for: 10, dex: 14, con: 13, int: 9, sag: 10, cha: 8 },
    ca: 12,
    items: [
      { name: "Mousquet court", desc: "Arme à feu compacte, bonne portée et discret", qty: 1 },
      { name: "Kit de pièges", desc: "Pièges à poser pour embuscades ou capture", qty: 2 }
    ],
    attacks: [
      {
        name: "Tir furtif",
        lvl: 1,
        desc: "Tir discret depuis l'ombre. Bonus si non repéré.",
        tags: [
          { type: "dmg", value: "3 dégâts (+2 si furtif)" },
          { type: "prec", value: "+2 précision" },
          { type: "cd", value: "Pas de cooldown" }
        ]
      },
      {
        name: "Piège à mâchoires",
        lvl: 3,
        desc: "Piège qui immobilise le premier ennemi.",
        tags: [
          { type: "dmg", value: "2 dégâts" },
          { type: "ctrl", value: "Immobilise 2 tours (jet FOR)" },
          { type: "cd", value: "CD: 3 tours" }
        ]
      },
      {
        name: "Embuscade mortelle",
        lvl: 7,
        desc: "Disparaît et frappe depuis les ombres. Critique garanti.",
        tags: [
          { type: "dmg", value: "8 dégâts (critique auto)" },
          { type: "ctrl", value: "Désoriente -2 précision (2 tours)" },
          { type: "cd", value: "CD: 5 tours" }
        ]
      }
    ]
  }
};
```

---

## Calculs et logique métier

### Modificateur de stat
```
modificateur = Math.floor((score - 10) / 2)
```

### Classe d'Armure (CA)
```
CA = 10 + modificateur de DEX
```

### Système de niveau
```
Niveau basé sur le total de XP accumulé :
- 0-4 XP   → Niveau 1
- 5-9 XP   → Niveau 2
- 10-14 XP → Niveau 3
- 15-19 XP → Niveau 4
- 20-24 XP → Niveau 5
- 25-29 XP → Niveau 6
- 30+ XP   → Niveau 7
```

### Système d'XP et upgrade
- XP courant va de 0 à 5 (barre)
- À 5 XP, le compteur reset à 0 et le joueur gagne +1 point d'amélioration
- Un point d'amélioration = +1 dans une stat au choix du joueur
- Le total XP (cumulé) détermine le niveau

### Attaques débloquées par niveau
- Les attaques de Nv.1 sont disponibles dès le départ
- Les attaques de Nv.3 se débloquent au niveau 3
- Les attaques de Nv.7 se débloquent au niveau 7
- + 1 slot libre pour une attaque apprise en jeu

---

## Design et UI — Reproduire le style existant

### Thème visuel
- **Background** : `#1a1410` (brun très sombre)
- **Surface** : gradient `#2a2218` → `#1e1a14`, bordure `#8b7355`
- **Texte principal** : `#e8dcc8` (parchemin clair)
- **Titres** : `#d4a847` (or) — font `MedievalSharp` (Google Fonts)
- **Texte secondaire** : `#8b7355` (brun moyen)
- **Corps de texte** : font `Crimson Text` (Google Fonts)
- **Inputs** : fond `rgba(0,0,0,0.3)`, bordure `#4a3f2f`, focus → bordure `#d4a847`

### Layout de la fiche
La fiche est structurée en sections verticales dans un conteneur max 800px :

1. **Header** — "Fiche de Personnage" + sous-titre session + version
2. **Section Classe** — sélecteur dropdown, info-box (description, stats principales, objets)
3. **Section Identité** — grid 2 colonnes : nom perso, région d'origine, puis full-width : description physique (textarea), background/histoire (textarea)
4. **Section Vitalité & Expérience** :
   - Bouclier CA centré (forme shield, valeur en gros + détail "10 + X")
   - Barre HP (rouge, `#c0392b`) avec boutons +/-
   - Barre XP (verte, `#27ae60`) avec boutons +/-
   - Affichage points d'amélioration + niveau
5. **Section Caractéristiques** — grid 6 colonnes :
   - Chaque stat : nom (MedievalSharp), input score, modificateur calculé, description courte
   - Les stats principales de la classe ont un highlight doré (border + background)
6. **Section Attaques** — cartes verticales :
   - Chaque attaque : header (nom + badge niveau), description italique, tags colorés
   - Tags par type : `dmg` rouge `#e74c3c`, `heal` vert `#27ae60`, `buff` bleu `#3498db`, `ctrl` orange `#f39c12`, `cd` brun `#8b7355`, `prec` violet `#9b59b6`
   - Cartes locked (dashed border, opacity 0.5) vs unlocked (bordure verte)
   - Slot libre en bas (inputs libres)
7. **Section Inventaire** — tableau (objet, description, qté) + or + bouton ajouter
8. **Section Notes** — textarea libre
9. **Bouton Save** — vert, centré

### Responsive
- En dessous de 600px : stats en grid 3 colonnes, barres HP/XP empilées, identité en 1 colonne

---

## Pages de l'application

### Page 1 : Accueil
- Titre "Corsaires de la Couronne"
- Deux boutons : "Créer une session (MJ)" / "Rejoindre une session (Joueur)"

### Page 2 : Création de session (MJ)
- Le MJ entre son nom
- Choix du template (pour l'instant un seul : "Corsaires de la Couronne")
- → Génère un code de session (6 chars alphanumériques)
- Redirige vers le Lobby MJ

### Page 3 : Lobby MJ
- Affiche le code de session en gros (copiable)
- Liste des joueurs connectés (temps réel)
- Pour chaque joueur : nom, classe choisie (ou "en attente"), statut
- Bouton "Lancer la partie" → passe en mode "in_game"

### Page 4 : Rejoindre (Joueur)
- Input : code de session + prénom du joueur
- Validation → redirige vers la fiche de personnage

### Page 5 : Fiche de personnage (Joueur)
- La fiche complète telle que décrite ci-dessus
- **En lobby** : le joueur peut choisir sa classe et remplir son identité
- **En jeu** : le joueur ne peut modifier que ses notes et son slot d'attaque libre
- Les modifications du MJ apparaissent en temps réel (animation flash sur les valeurs changées)

### Page 6 : Dashboard MJ (en jeu)
- Vue d'ensemble de tous les joueurs (mini-fiches avec HP/CA/classe)
- Clic sur un joueur → ouvre sa fiche en mode édition complète
- Actions rapides :
  - "+1 XP à [joueur]"
  - "-X HP à [joueur]"
  - "+X HP à [joueur]"
  - "Ajouter objet à [joueur]"
  - "Modifier or de [joueur]"
- Historique des modifications (log)

---

## API Endpoints (suggestion)

```
POST   /api/sessions              → créer une session
GET    /api/sessions/:code        → infos session (public, pour rejoindre)
POST   /api/sessions/:code/join   → joueur rejoint {name}
GET    /api/sessions/:id/players  → liste joueurs + statut
PATCH  /api/sessions/:id/status   → changer statut (lobby → in_game → finished)

GET    /api/sheets/:id            → récupérer une fiche
PATCH  /api/sheets/:id            → update partiel d'une fiche
GET    /api/sessions/:id/sheets   → toutes les fiches d'une session (MJ only)

WebSocket /ws/session/:id         → canal temps réel (updates fiches, joueurs qui rejoignent)
```

---

## Permissions

| Action | Joueur (lobby) | Joueur (in_game) | MJ |
|--------|---------------|-----------------|-----|
| Choisir classe | ✅ | ❌ | ✅ |
| Éditer identité | ✅ | ❌ | ✅ |
| Voir stats | ✅ | ✅ | ✅ |
| Modifier stats | ❌ | ❌ | ✅ |
| Modifier HP/XP | ❌ | ❌ | ✅ |
| Modifier inventaire | ❌ | ❌ | ✅ |
| Éditer notes | ✅ | ✅ | ✅ |
| Éditer slot attaque libre | ✅ | ✅ | ✅ |
| Voir toutes les fiches | ❌ | ❌ | ✅ |

---

## Fonctionnalités bonus (nice-to-have)

- **Lanceur de dés intégré** : bouton "d20" sur la fiche, affiche le résultat + modificateur
- **Timer de cooldown** : les attaques utilisées montrent un compteur de tours restants
- **Historique des jets** : log visible par le MJ de tous les jets faits par les joueurs
- **Mode sombre/clair** : toggle (mais le thème pirate est déjà sombre)
- **Export PDF** de la fiche en fin de session
- **Chat intégré** : canal textuel MJ ↔ joueurs

---

## Notes techniques

- L'auth peut être ultra-simple : le MJ a un token en cookie, les joueurs sont identifiés par session_code + prénom (pas de compte)
- La DB peut être SQLite pour un one-shot, ça tient sur un fichier
- Le WebSocket peut être remplacé par du polling toutes les 2s si plus simple à implémenter
- Le template "Corsaires de la Couronne" est hardcodé, mais la structure permet d'ajouter d'autres templates plus tard
- Penser au mobile : les joueurs seront probablement sur téléphone

---

## CSS des tags d'attaque (référence exacte)

```css
.atk-tag { font-size: 0.75em; padding: 3px 8px; border-radius: 6px; background: rgba(0,0,0,0.3); }
.atk-tag.dmg { color: #e74c3c; }   /* dégâts — rouge */
.atk-tag.heal { color: #27ae60; }  /* soin — vert */
.atk-tag.buff { color: #3498db; }  /* buff — bleu */
.atk-tag.ctrl { color: #f39c12; }  /* contrôle — orange */
.atk-tag.cd { color: #8b7355; }    /* cooldown — brun */
.atk-tag.prec { color: #9b59b6; }  /* précision — violet */
```

---

## Résumé

Ce document contient tout ce qu'il faut pour reproduire l'expérience des fiches HTML existantes sous forme d'application web serveur :
- Les données complètes des 8 classes (stats, objets, attaques avec tags)
- La logique de jeu (modificateurs, CA, XP, niveaux, attaques level-gated)
- Le design visuel exact (couleurs, fonts, layout, responsive)
- L'architecture session/joueur/MJ avec permissions
- Les flux utilisateurs et pages nécessaires
