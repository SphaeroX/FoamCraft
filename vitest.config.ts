// SPDX-License-Identifier: GPL-3.0-or-later
import react from "@vitejs/plugin-react";
import { defineConfig } from "vitest/config";

export default defineConfig({
  plugins: [react()],
  test: {
    // Frontend tests live in the repository-level tests/ folder, not next to the sources.
    include: ["tests/app/**/*.test.{ts,tsx}"],
    setupFiles: ["tests/app/setup.ts"],
  },
});
