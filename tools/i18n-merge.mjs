// SPDX-License-Identifier: GPL-3.0-or-later
// Adds or changes interface texts in app/src/i18n/en.json and de.json in one step, so both
// languages always get the same keys.
//
// Usage: node tools/i18n-merge.mjs <file.json>
// The file holds dotted keys per language:
//   { "en": { "run.meshOnly": "Mesh only" }, "de": { "run.meshOnly": "Nur vernetzen" } }
/* global console, process */
import { readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const input = process.argv[2];
if (!input) {
  console.error("usage: node tools/i18n-merge.mjs <file.json>");
  process.exit(2);
}
const add = JSON.parse(readFileSync(input, "utf8"));
const en = Object.keys(add.en ?? {}).sort();
const de = Object.keys(add.de ?? {}).sort();
if (en.length === 0 || en.join() !== de.join()) {
  console.error('the file needs the same keys under "en" and "de"');
  process.exit(1);
}
const folder = join(dirname(fileURLToPath(import.meta.url)), "..", "app", "src", "i18n");
for (const lang of ["en", "de"]) {
  const file = join(folder, `${lang}.json`);
  const doc = JSON.parse(readFileSync(file, "utf8"));
  for (const [key, value] of Object.entries(add[lang])) {
    const parts = key.split(".");
    let node = doc;
    for (const part of parts.slice(0, -1)) {
      if (typeof node[part] !== "object" || node[part] === null) node[part] = {};
      node = node[part];
    }
    node[parts.at(-1)] = value;
  }
  writeFileSync(file, `${JSON.stringify(doc, null, 2)}\n`);
}
console.log(`merged ${en.length} keys into en.json and de.json`);
