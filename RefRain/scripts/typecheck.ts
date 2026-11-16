// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { join } from "node:path";

const ROOT = join(import.meta.dir, "..");

interface Lane {
  readonly cmd: readonly string[];
  readonly clean: RegExp;
}

const lanes: readonly Lane[] = [
  { cmd: ["bun", "check", "--no-script"], clean: /No type errors in \d+ files/ },
  { cmd: ["bunx", "svelte-check", "--no-tsconfig"], clean: /0 errors and 0 warnings/ },
];

let failed = false;
for (const lane of lanes) {
  const run = Bun.spawnSync([...lane.cmd], {
    cwd: ROOT,
    stdout: "pipe",
    stderr: "pipe",
  });
  const output = run.stdout.toString() + run.stderr.toString();
  console.log(output);
  const isClean = run.exitCode === 0 && lane.clean.test(output);
  if (!isClean) failed = true;
}
process.exit(failed ? 1 : 0);
