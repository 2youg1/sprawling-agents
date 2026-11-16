// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The typecheck lane, sealed (client/Spec.lean §4-1): two complements,
// each run and each sealed, because a toolchain that failed to load
// must not read as clean.
//
// `bun check` is the TypeScript 7 checker built into the runtime: it
// reads this project's tsconfig and owns every `.ts` file, all cores,
// no writes. `.svelte` files are the one shape it cannot read, and
// svelte-check owns them: run with `--no-tsconfig` it checks every
// component and the TS inside each script block, with the same strict
// flags (probed: a `noUncheckedIndexedAccess` violation inside a
// component is reported), while standalone `.ts` files are not
// type-checked - so the two halves do not overlap and leave no hole.
//
// Each run is captured and counts as clean only when two assertions
// hold: exit code 0 and the lane's clean summary line in its output.
// A lane that printed an error and stopped fails its assertions, and
// any failure prints the captured output and exits 1.

import { join } from "node:path";

// The client root, so the lanes find this tsconfig whatever directory
// the caller started from.
const ROOT = join(import.meta.dir, "..");

interface Lane {
  readonly cmd: readonly string[];
  readonly clean: RegExp;
}

const lanes: readonly Lane[] = [
  // `--no-script`: the type checker even if a `check` script ever lands
  // in package.json (`bun check` would otherwise run that script).
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
