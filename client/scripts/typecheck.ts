// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The typecheck lane, sealed (client/Spec.lean §4-1). `svelte-check --tsgo`
// reads `.svelte` and `.ts` in one pass on the TypeScript 7 checker
// that `@typescript/native` installs, but the lane has one measured
// defect: when its setup fails - TypeScript 7 missing from the
// workspace - it prints an error and still exits 0. A gate that can
// pass while broken is not evidence, so this wrapper runs the lane
// with its output captured and forwards the run only when three
// assertions hold: the exit code is 0, the summary reports
// "0 errors and 0 warnings", and the output carries neither the lane's
// "svelte-check failed" banner nor its "requires TypeScript 7 to be
// installed" setup-failure sentence. Any assertion failing prints the
// captured output and exits 1.
//
// The lane also writes a transpiled project to disk, which would make
// a run answer for the previous one; the directory is deleted before
// every run, so the check reads the tree alone. `svelte-check` runs
// here through `bun x`, the same invocation `bunx` names.

import { rmSync } from "node:fs";
import { join } from "node:path";

// The client root. Every path below hangs off this file's own
// directory, so the lane finds the same tsconfig whatever directory
// the caller started from.
const ROOT = join(import.meta.dir, "..");

// Where the `--tsgo` lane writes its transpiled project. Observed
// beside `tsconfig.json` at the client root, never under `src/`.
const LANE_SCRATCH = join(ROOT, ".svelte-check");

rmSync(LANE_SCRATCH, { recursive: true, force: true });

const run = Bun.spawnSync(["bunx", "svelte-check", "--tsconfig", "./tsconfig.json", "--tsgo"], {
  cwd: ROOT,
  stdout: "pipe",
  stderr: "pipe",
});
const output = run.stdout.toString() + run.stderr.toString();
console.log(output);

// The lane scratch is deleted after the run as well: the gates walk
// the tree on disk, and generated Svelte internals read as findings
// nobody wrote (the secret gate alone reported hundreds).
rmSync(LANE_SCRATCH, { recursive: true, force: true });

const exitIsZero = run.exitCode === 0;
const summaryIsClean = output.includes("0 errors and 0 warnings");
const setupSucceeded =
  !output.includes("svelte-check failed") &&
  !output.includes("requires TypeScript 7 to be installed");

if (!(exitIsZero && summaryIsClean && setupSucceeded)) {
  process.exit(1);
}
