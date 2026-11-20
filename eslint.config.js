// SPDX-License-Identifier: GPL-3.0-or-later
import js from "@eslint/js";
import tseslint from "typescript-eslint";

export default tseslint.config(
  {
    ignores: ["**/dist/**", "**/target/**", "**/node_modules/**", "app/src-tauri/gen/**"],
  },
  js.configs.recommended,
  ...tseslint.configs.recommended,
);
