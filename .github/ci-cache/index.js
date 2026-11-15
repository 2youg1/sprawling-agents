// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The runner gives Node actions the cache service environment. Inherit it
// directly; neither an environment file nor a log should carry its token.
import { spawnSync } from "node:child_process";
import { join } from "node:path";

if (!process.env.GITHUB_WORKSPACE) {
  console.error("source verification requires GITHUB_WORKSPACE (verification-workspace)");
  process.exitCode = 2;
} else {
  const result = spawnSync("just", ["verify-sources"], {
    cwd: process.env.GITHUB_WORKSPACE,
    env: {
      ...process.env,
      SPRAWLING_CI_CACHE_ADAPTER: join(import.meta.dirname, "cache.ts"),
    },
    stdio: "inherit",
  });
  if (result.error) {
    console.error(`could not launch source verification (verification-launch): ${result.error}`);
  } else if (result.signal) {
    console.error(`source verification terminated by ${result.signal} (verification-signal)`);
  }
  process.exitCode = result.status ?? 2;
}
