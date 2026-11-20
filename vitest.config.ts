// SPDX-License-Identifier: GPL-3.0-or-later
import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    // Frontend tests live in the repository-level tests/ folder, not next to the sources.
    include: ["tests/app/**/*.test.{ts,tsx}"],
  },
});
