/// <reference types="vitest/config" />
import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";

// Tauri serves the dev build from a fixed port and never through a public host.
export default defineConfig({
  plugins: [svelte()],
  clearScreen: false,
  server: { port: 1420, strictPort: true, host: "127.0.0.1" },
  // The windows render the core's Scene and own no copy (the strings live in the core's
  // copy.rs, linted and checked against the spec there): no unit tests here today. The
  // galleries and drives (npm run test:gallery, drive:*) are the webviews' tests.
  test: { passWithNoTests: true },
  build: {
    target: "safari16",
    outDir: "dist",
    emptyOutDir: true,
    // pattern.html is the dev harness's test pattern (TESTING-PLAN §4).
    rollupOptions: { input: { main: "index.html", pattern: "pattern.html", popover: "popover.html", countdown: "countdown.html", onboarding: "onboarding.html", library: "library.html", settings: "settings.html" } },
  },
});
