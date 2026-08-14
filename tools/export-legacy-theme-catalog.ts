// Build-time migration helper. Run from this repository with TSX_TSCONFIG_PATH
// pointed at the legacy application's tsconfig so its @/ aliases resolve.
import { writeFileSync } from "node:fs";
import {
  BUILT_IN_THEMES,
  getThemeData,
} from "../../buttonscli-oss1/src/data/builtInThemes.ts";

const output = process.argv[2];
if (!output) {
  throw new Error("usage: export-legacy-theme-catalog.ts OUTPUT.json");
}

const catalog = BUILT_IN_THEMES.map((metadata) => ({
  version: 1,
  metadata,
  theme: getThemeData(metadata.id),
}));

writeFileSync(output, `${JSON.stringify(catalog, null, 2)}\n`);
console.log(`exported ${catalog.length} legacy code themes to ${output}`);
