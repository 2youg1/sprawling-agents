// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// Compiles a `.svelte.ts` module for `bun test` the way vite compiles it
// for the bundle: TypeScript stripped, then `compileModule` turning the
// runes into the client runtime's calls. Without it a test importing a
// module that holds `$state` meets an undefined global instead of the
// reactivity the page runs on.

import { plugin } from "bun";
import { compileModule } from "svelte/compiler";

// The page runs svelte's browser build, and so must a test: under the
// server build `$effect` never runs and a test of an effect fails on an
// assertion that says nothing about why. Bun takes the `browser`
// condition only as the `--conditions` flag - `bunfig.toml` has no key
// for it - so a run without the flag stops here and names the command
// that carries it, instead of testing the server build.
if (!Bun.resolveSync("svelte", import.meta.dir).replaceAll("\\", "/").endsWith("/index-client.js")) {
  console.error(
    "bun test resolved svelte's server build: run the tests through `bun run test [file...]`, " +
      "which passes `--conditions=browser`",
  );
  process.exit(1);
}

plugin({
  name: "svelte runes",
  setup(build) {
    const strip = new Bun.Transpiler({ loader: "ts" });
    build.onLoad({ filter: /\.svelte(\.test)?\.ts$/ }, async ({ path }) => {
      const script = strip.transformSync(await Bun.file(path).text());
      const { js } = compileModule(script, { filename: path, generate: "client" });
      return { contents: js.code, loader: "js" };
    });
  },
});
