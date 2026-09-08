// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import tailwindcss from "@tailwindcss/vite";
import { defineConfig } from "vite";
import solid from "vite-plugin-solid";

// The bundle lands where `crates/sprawling/build.rs` reads it: the
// directory named `web-dist` under the workspace's `target/`, with
// `index.html` at its root. `base: './'` keeps every asset reference
// relative, so the same bundle serves from inside the binary.
export default defineConfig({
  root: "src",
  base: "./",
  plugins: [tailwindcss(), solid()],
  build: {
    outDir: "../../target/web-dist",
    emptyOutDir: true,
  },
});
