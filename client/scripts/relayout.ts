// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The relayout probe (xtask/budgets.toml `stream_relayout`): what one
// streamed token costs the page in script plus forced layout, for the
// raw shape a live turn used to draw and the block shape
// `talk/saying.svelte` draws now, and how far the turn jumps when the
// record takes over. Run with `bun scripts/relayout.ts [browser]`; the
// browser defaults to Edge, the engine `xtask render` also prefers.
// The page is built from `relayout_page.ts` and loaded headless with
// `--dump-dom`, which prints the page after its script has run; the page
// reports its own summary, so this half only builds, runs and prints.

import { mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { pathToFileURL } from "node:url";

const EDGE = "C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe";

const built = await Bun.build({
  entrypoints: [join(import.meta.dir, "relayout_page.ts")],
  target: "browser",
  format: "iife",
});
const script = (await built.outputs[0]?.text()) ?? "";
const page = join(mkdtempSync(join(tmpdir(), "relayout-")), "page.html");
writeFileSync(page, `<!doctype html><html><body><script>${script}</script></body></html>`);

const browser = process.argv[2] ?? EDGE;
const run = Bun.spawnSync([browser, "--headless=new", "--disable-gpu", "--dump-dom", pathToFileURL(page).href]);
const dumped = /<pre id="relayout">([^<]*)<\/pre>/.exec(run.stdout.toString());
if (dumped === null || script === "") {
  console.error(`no reading came back from ${browser}:\n${run.stderr.toString()}`);
  process.exit(1);
}
console.log((dumped[1] ?? "").replaceAll("&amp;", "&"));
