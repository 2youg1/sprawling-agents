// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { existsSync, readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

import tailwindcss from "@tailwindcss/vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";
import { defineConfig } from "vite";
import type { Plugin } from "vite";

import { thirdPartyNotices } from "./scripts/notices";

// The files the shipped faces are made of: Geist Mono's one variable
// woff2, Libron's four static ones, and the licence each is given
// under. `theme/fonts.css` names every woff2 file, so the asset pipeline
// emits them by itself; the licences are named by nothing, and OFL-1.1
// requires each to travel with its font, so this plugin puts them in the
// bundle beside them.
const FACES = [
  "GeistMono-Variable.woff2",
  "Libron-Regular.woff2",
  "Libron-Italic.woff2",
  "Libron-Bold.woff2",
  "Libron-BoldItalic.woff2",
];
const LICENCES = ["OFL.txt", "Libron-OFL.txt"];

// A face that is not in the tree is not a broken build: the fallback
// chain draws the page with what the machine has. It is still a bundle
// nobody should ship without noticing, so each missing file is named
// once per build, and the warning clears itself the moment the file is
// dropped in.
function shippedFace(): Plugin {
  const at = (name: string) => fileURLToPath(new URL(`./src/fonts/${name}`, import.meta.url));
  return {
    name: "sprawling:shipped-face",
    apply: "build",
    generateBundle() {
      for (const name of FACES) {
        if (!existsSync(at(name))) {
          this.warn(`client/src/fonts/${name} is absent; the page falls back to the system stack`);
        }
      }
      for (const name of LICENCES) {
        const path = at(name);
        if (existsSync(path)) {
          this.emitFile({ type: "asset", fileName: `fonts/${name}`, source: readFileSync(path) });
        } else {
          this.warn(`client/src/fonts/${name} is absent; a font may not be shipped without it`);
        }
      }
    },
  };
}

// The bundle lands where `crates/sprawling/build.rs` reads it: the path
// its `BUNDLE_DIR` states inside its own package, crates/sprawling/web-dist,
// whatever `CARGO_TARGET_DIR` says, with `index.html` at its root; inside
// the package because the crates.io archive carries nothing else.
// `base: './'` keeps every asset reference relative, so the same bundle
// serves from inside the binary.
//
// `configFile` is load-bearing: the plugin resolves `svelte.config.*`
// against `root` (here `src`), while the config lives at the project
// root beside this file — the one place `svelte-check` also finds it.
export default defineConfig({
  root: "src",
  base: "./",
  plugins: [
    tailwindcss(),
    svelte({ configFile: "../svelte.config.ts" }),
    shippedFace(),
    thirdPartyNotices(),
  ],
  build: {
    outDir: "../../crates/sprawling/web-dist",
    emptyOutDir: true,
    // No sourcemap ships: the map of this bundle is 4 MB against a
    // 644 KB bundle, `xtask budget` weighs everything the directory
    // holds, and `build.rs` embeds every byte of it in the binary.
    sourcemap: false,
  },
});
