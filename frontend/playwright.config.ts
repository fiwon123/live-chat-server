import { defineConfig } from "@playwright/test";

// Shared Playwright settings.
//
// The screenshot harness (`scripts/screenshots.mjs`) is a plain Node script
// and mirrors these defaults (same origin env var, same viewport) rather than
// importing this file; this config exists for future `e2e/` test files and as
// the single source of the canonical capture settings.
export default defineConfig({
  testDir: "e2e",
  timeout: 60_000,
  expect: { timeout: 10_000 },
  fullyParallel: false,
  reporter: "list",
  use: {
    baseURL: process.env.VITE_ORIGIN ?? "http://localhost:5173",
    viewport: { width: 1280, height: 800 },
    colorScheme: "dark",
  },
});
