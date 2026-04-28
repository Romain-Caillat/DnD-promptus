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
    name: "Tavern bustle",
    type: "ambience",
    filePath: "https://cdn.pixabay.com/audio/2022/03/15/audio_d5b37cd4f5.mp3",
    durationSeconds: 137,
    tags: ["tavern", "village", "crowd", "indoor"],
    attribution: "Pixabay — Tavern Background by mellauria",
  },
  {
    name: "Forest at night",
    type: "ambience",
    filePath: "https://cdn.pixabay.com/audio/2024/02/12/audio_a48a2317d8.mp3",
    durationSeconds: 180,
    tags: ["forest", "night", "outdoor", "wilderness"],
    attribution: "Pixabay — Forest Night Ambience",
  },
  {
    name: "Crypt drips",
    type: "ambience",
    filePath: "https://cdn.pixabay.com/audio/2022/03/15/audio_e91e92a4ba.mp3",
    durationSeconds: 145,
    tags: ["crypt", "dungeon", "spooky", "indoor", "underground"],
    attribution: "Pixabay — Cave Water Drops",
  },
  {
    name: "Castle hall",
    type: "ambience",
    filePath: "https://cdn.pixabay.com/audio/2023/05/05/audio_2ad7b04d09.mp3",
    durationSeconds: 120,
    tags: ["castle", "indoor", "noble", "echo"],
    attribution: "Pixabay — Medieval Hall",
  },
  {
    name: "Marketplace",
    type: "ambience",
    filePath: "https://cdn.pixabay.com/audio/2022/05/16/audio_b8a59e53c2.mp3",
    durationSeconds: 130,
    tags: ["market", "town", "crowd", "outdoor"],
    attribution: "Pixabay — Medieval Market",
  },
  {
    name: "Wind in mountains",
    type: "ambience",
    filePath: "https://cdn.pixabay.com/audio/2022/03/24/audio_3a2bf9d7a5.mp3",
    durationSeconds: 160,
    tags: ["wind", "mountain", "outdoor", "travel"],
    attribution: "Pixabay — Mountain Wind",
  },
  {
    name: "Campfire crackle",
    type: "ambience",
    filePath: "https://cdn.pixabay.com/audio/2022/03/10/audio_270f49b83e.mp3",
    durationSeconds: 125,
    tags: ["campfire", "rest", "outdoor", "night"],
    attribution: "Pixabay — Campfire Sounds",
  },
  {
    name: "Underground dungeon",
    type: "ambience",
    filePath: "https://cdn.pixabay.com/audio/2024/05/22/audio_98aafcd9e7.mp3",
    durationSeconds: 180,
    tags: ["dungeon", "indoor", "underground", "torch"],
    attribution: "Pixabay — Dungeon Atmosphere",
  },

  // Music
  {
    name: "Heroic combat",
    type: "music",
    filePath: "https://cdn.pixabay.com/audio/2024/02/05/audio_d05d9bc6c5.mp3",
    durationSeconds: 156,
    tags: ["combat", "tense", "heroic", "epic"],
    attribution: "Pixabay — Epic Battle",
  },
  {
    name: "Boss confrontation",
    type: "music",
    filePath: "https://cdn.pixabay.com/audio/2023/09/06/audio_e6f63b5a37.mp3",
    durationSeconds: 200,
    tags: ["combat", "boss", "climax", "epic"],
    attribution: "Pixabay — Boss Fight",
  },
  {
    name: "Mystery exploration",
    type: "music",
    filePath: "https://cdn.pixabay.com/audio/2023/03/30/audio_8b1be9b2e2.mp3",
    durationSeconds: 170,
    tags: ["exploration", "mystery", "calm", "ambient"],
    attribution: "Pixabay — Mystery Exploration",
  },
  {
    name: "Tense dialogue",
    type: "music",
    filePath: "https://cdn.pixabay.com/audio/2022/10/30/audio_1c5e3e1cab.mp3",
    durationSeconds: 140,
    tags: ["dialogue", "tense", "drama"],
    attribution: "Pixabay — Tension Drama",
  },
  {
    name: "Victory fanfare",
    type: "music",
    filePath: "https://cdn.pixabay.com/audio/2022/02/22/audio_1f5b4d7027.mp3",
    durationSeconds: 32,
    tags: ["victory", "fanfare", "celebration"],
    attribution: "Pixabay — Victory Fanfare",
  },

  // Sound effects
  {
    name: "Sword clash",
    type: "sound",
    filePath: "https://cdn.pixabay.com/audio/2022/03/10/audio_3eaf24cc14.mp3",
    durationSeconds: 2,
    tags: ["sword", "combat", "metal"],
    attribution: "Pixabay — Sword Clash",
  },
  {
    name: "Door creaking",
    type: "sound",
    filePath: "https://cdn.pixabay.com/audio/2022/03/24/audio_29c1d1f74e.mp3",
    durationSeconds: 4,
    tags: ["door", "wood", "creak"],
    attribution: "Pixabay — Door Creak",
  },
  {
    name: "Spell sparkle",
    type: "sound",
    filePath: "https://cdn.pixabay.com/audio/2022/03/24/audio_28b4f8a21e.mp3",
    durationSeconds: 3,
    tags: ["spell", "magic", "sparkle"],
    attribution: "Pixabay — Magic Spell",
  },
  {
    name: "Coin pouch",
    type: "sound",
    filePath: "https://cdn.pixabay.com/audio/2022/01/26/audio_75e7dba47b.mp3",
    durationSeconds: 2,
    tags: ["coin", "loot", "gold"],
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
