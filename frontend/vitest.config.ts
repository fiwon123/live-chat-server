import { defineConfig } from "vitest/config";

// Unit tests cover the pure client logic (URL building, error parsing, message
// merging) with no DOM, so `node` keeps them fast and dependency-light.
export default defineConfig({
  test: {
    environment: "node",
    include: ["src/**/*.test.ts"],
  },
});
