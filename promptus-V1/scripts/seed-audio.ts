import { config } from "dotenv";
config({ path: ".env.local" });

import { db } from "../src/lib/db/client";
import { audioAssets } from "../src/lib/db/schema";
import { generateId } from "../src/lib/api/ids";
import { eq } from "drizzle-orm";

/**
 * Seed audio catalog with royalty-free Pixabay sources.
 * URLs are streamed directly by the <audio> tag — no local download needed
 * for MVP. To go fully self-hosted, run a fetch script that pulls each URL
 * to /public/audio/{id}.mp3 and rewrites filePath.
 *
 * License: Pixabay Content License (commercial use OK, no attribution
 * required but appreciated). https://pixabay.com/service/terms/
 */

interface SeedAsset {
  name: string;
  type: "ambience" | "music" | "sound";
  filePath: string;
  durationSeconds: number;
  tags: string[];
  attribution?: string;
}

const ASSETS: SeedAsset[] = [
  // Ambiences
  {
    name: "Brouhaha de taverne",
    type: "ambience",
    filePath: "https://cdn.pixabay.com/audio/2022/03/15/audio_d5b37cd4f5.mp3",
    durationSeconds: 137,
    tags: ["taverne", "village", "foule", "intérieur"],
    attribution: "Pixabay — Tavern Background by mellauria",
  },
  {
    name: "Forêt de nuit",
    type: "ambience",
    filePath: "https://cdn.pixabay.com/audio/2024/02/12/audio_a48a2317d8.mp3",
    durationSeconds: 180,
    tags: ["forêt", "nuit", "extérieur", "nature"],
    attribution: "Pixabay — Forest Night Ambience",
  },
  {
    name: "Gouttes dans la crypte",
    type: "ambience",
    filePath: "https://cdn.pixabay.com/audio/2022/03/15/audio_e91e92a4ba.mp3",
    durationSeconds: 145,
    tags: ["crypte", "donjon", "angoissant", "intérieur", "souterrain"],
    attribution: "Pixabay — Cave Water Drops",
  },
  {
    name: "Grand hall de château",
    type: "ambience",
    filePath: "https://cdn.pixabay.com/audio/2023/05/05/audio_2ad7b04d09.mp3",
    durationSeconds: 120,
    tags: ["château", "intérieur", "noble", "écho"],
    attribution: "Pixabay — Medieval Hall",
  },
  {
    name: "Place du marché",
    type: "ambience",
    filePath: "https://cdn.pixabay.com/audio/2022/05/16/audio_b8a59e53c2.mp3",
    durationSeconds: 130,
    tags: ["marché", "ville", "foule", "extérieur"],
    attribution: "Pixabay — Medieval Market",
  },
  {
    name: "Vent des montagnes",
    type: "ambience",
    filePath: "https://cdn.pixabay.com/audio/2022/03/24/audio_3a2bf9d7a5.mp3",
    durationSeconds: 160,
    tags: ["vent", "montagne", "extérieur", "voyage"],
    attribution: "Pixabay — Mountain Wind",
  },
  {
    name: "Crépitement du feu de camp",
    type: "ambience",
    filePath: "https://cdn.pixabay.com/audio/2022/03/10/audio_270f49b83e.mp3",
    durationSeconds: 125,
    tags: ["feu de camp", "repos", "extérieur", "nuit"],
    attribution: "Pixabay — Campfire Sounds",
  },
  {
    name: "Donjon souterrain",
    type: "ambience",
    filePath: "https://cdn.pixabay.com/audio/2024/05/22/audio_98aafcd9e7.mp3",
    durationSeconds: 180,
    tags: ["donjon", "intérieur", "souterrain", "torche"],
    attribution: "Pixabay — Dungeon Atmosphere",
  },

  // Music
  {
    name: "Combat héroïque",
    type: "music",
    filePath: "https://cdn.pixabay.com/audio/2024/02/05/audio_d05d9bc6c5.mp3",
    durationSeconds: 156,
    tags: ["combat", "tendu", "héroïque", "épique"],
    attribution: "Pixabay — Epic Battle",
  },
  {
    name: "Affrontement final",
    type: "music",
    filePath: "https://cdn.pixabay.com/audio/2023/09/06/audio_e6f63b5a37.mp3",
    durationSeconds: 200,
    tags: ["combat", "boss", "apogée", "épique"],
    attribution: "Pixabay — Boss Fight",
  },
  {
    name: "Exploration mystérieuse",
    type: "music",
    filePath: "https://cdn.pixabay.com/audio/2023/03/30/audio_8b1be9b2e2.mp3",
    durationSeconds: 170,
    tags: ["exploration", "mystère", "calme", "ambiance"],
    attribution: "Pixabay — Mystery Exploration",
  },
  {
    name: "Dialogue tendu",
    type: "music",
    filePath: "https://cdn.pixabay.com/audio/2022/10/30/audio_1c5e3e1cab.mp3",
    durationSeconds: 140,
    tags: ["dialogue", "tendu", "drame"],
    attribution: "Pixabay — Tension Drama",
  },
  {
    name: "Fanfare de victoire",
    type: "music",
    filePath: "https://cdn.pixabay.com/audio/2022/02/22/audio_1f5b4d7027.mp3",
    durationSeconds: 32,
    tags: ["victoire", "fanfare", "célébration"],
    attribution: "Pixabay — Victory Fanfare",
  },

  // Sound effects
  {
    name: "Choc d’épées",
    type: "sound",
    filePath: "https://cdn.pixabay.com/audio/2022/03/10/audio_3eaf24cc14.mp3",
    durationSeconds: 2,
    tags: ["épée", "combat", "métal"],
    attribution: "Pixabay — Sword Clash",
  },
  {
    name: "Porte qui grince",
    type: "sound",
    filePath: "https://cdn.pixabay.com/audio/2022/03/24/audio_29c1d1f74e.mp3",
    durationSeconds: 4,
    tags: ["porte", "bois", "grincement"],
    attribution: "Pixabay — Door Creak",
  },
  {
    name: "Scintillement magique",
    type: "sound",
    filePath: "https://cdn.pixabay.com/audio/2022/03/24/audio_28b4f8a21e.mp3",
    durationSeconds: 3,
    tags: ["sort", "magie", "scintillement"],
    attribution: "Pixabay — Magic Spell",
  },
  {
    name: "Bourse de pièces",
    type: "sound",
    filePath: "https://cdn.pixabay.com/audio/2022/01/26/audio_75e7dba47b.mp3",
    durationSeconds: 2,
    tags: ["pièce", "butin", "or"],
    attribution: "Pixabay — Coins",
  },
];

async function main() {
  console.log("[seed-audio] starting...");

  // Wipe existing audio_assets so re-runs are idempotent.
  const existing = await db.select().from(audioAssets);
  console.log(`[seed-audio] removing ${existing.length} existing assets`);
  for (const e of existing) {
    await db.delete(audioAssets).where(eq(audioAssets.id, e.id));
  }

  for (const a of ASSETS) {
    const id = generateId(`audio_${a.type}`);
    await db.insert(audioAssets).values({
      id,
      name: a.name,
      type: a.type,
      filePath: a.filePath,
      durationSeconds: a.durationSeconds,
      tags: a.tags,
      license: "Pixabay Content License",
      attribution: a.attribution ?? null,
    });
    console.log(`[seed-audio] ${a.type.padEnd(8)} ${a.name}`);
  }

  console.log(`[seed-audio] done — ${ASSETS.length} assets seeded.`);
  process.exit(0);
}

main().catch((err) => {
  console.error("[seed-audio] failed:", err);
  process.exit(1);
});
