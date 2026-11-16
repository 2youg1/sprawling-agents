// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { plugin } from "bun";
import { compileModule } from "svelte/compiler";

if (
  !Bun.resolveSync("svelte", import.meta.dir)
    .replaceAll("\\", "/")
    .endsWith("/index-client.js")
) {
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
