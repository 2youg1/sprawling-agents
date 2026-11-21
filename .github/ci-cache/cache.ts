// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// Thin GitHub Actions cache adapter. Rust owns keys, validation and policy.
import { isFeatureAvailable, restoreCache, saveCache } from "@actions/cache";
import { resolve } from "node:path";

const [operation, key, directory] = process.argv.slice(2);
if (!operation || !key || !directory || process.argv.length !== 5) {
  console.error("usage: bun cache.ts <restore|save> <key> <directory>");
  process.exit(2);
}

const verdict = resolve(directory, "verdict.json");
if (operation === "restore") {
  if (!isFeatureAvailable()) {
    console.log("miss");
  } else {
    const matched = await restoreCache([verdict], key);
    console.log(matched === key ? "hit" : "miss");
  }
} else if (operation === "save") {
  if (isFeatureAvailable()) {
    await saveCache([verdict], key);
  }
  console.log("saved-or-unavailable");
} else {
  console.error(`unknown cache operation: ${operation}`);
  process.exit(2);
}
