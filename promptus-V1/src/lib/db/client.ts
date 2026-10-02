import { drizzle } from "drizzle-orm/node-postgres";
import { Pool } from "pg";
import * as schema from "./schema";

const connectionString =
  process.env.DATABASE_URL ??
  "postgresql://promptus:promptus_dev@localhost:5433/promptus";

const globalForDb = globalThis as unknown as {
  pool?: Pool;
};

const pool =
  globalForDb.pool ??
  new Pool({
    connectionString,
    max: 10,
  });

if (!globalForDb.pool) {
  // Connexion inactive coupée (Postgres redémarré) : la requête suivante en
  // ouvre une autre ; sans écouteur, l'erreur arrêterait le serveur.
  pool.on("error", (e) => console.error("[db] connexion perdue", e.message));
}

if (process.env.NODE_ENV !== "production") {
  globalForDb.pool = pool;
}

export const db = drizzle(pool, { schema });
export type DB = typeof db;
