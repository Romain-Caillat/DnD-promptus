import { defineConfig } from "vitest/config";
import path from "node:path";

const shared = {
  environment: "node" as const,
  globals: false,
  setupFiles: ["./vitest.setup.ts"],
};

export default defineConfig({
  resolve: {
    alias: {
      "@": path.resolve(__dirname, "./src"),
      "server-only": path.resolve(__dirname, "./test/stubs/server-only.ts"),
    },
  },
  test: {
    projects: [
      {
        extends: true,
        test: {
          ...shared,
          name: "unit",
          include: ["src/**/*.test.ts", "tests/unit/**/*.test.ts"],
        },
      },
      {
        // Requires Postgres (see docker-compose.yml + `pnpm db:migrate`).
        extends: true,
        test: {
          ...shared,
          name: "integration",
          include: ["tests/integration/**/*.test.ts"],
        },
      },
    ],
  },
});
