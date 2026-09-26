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
