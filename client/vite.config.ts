// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { existsSync, readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

import tailwindcss from "@tailwindcss/vite";
import { defineConfig } from "vite";
import type { Plugin } from "vite";
import solid from "vite-plugin-solid";

// The three files a shipped face is made of: one variable woff2 per
// family, and the licence they are given under. `theme.css` names the
// two woff2 files, so the asset pipeline emits those by itself; the
// licence is named by nothing, and OFL-1.1 requires it to travel with
// the font, so this plugin puts it in the bundle beside them.
const SANS = "Geist-Variable.woff2";
const MONO = "GeistMono-Variable.woff2";
const LICENCE = "OFL.txt";

// A face that is not in the tree is not a broken build: `font-display:
// swap` and the fallback chain draw the page with what the machine has.
// It is still a bundle nobody should ship without noticing, so each
// missing file is named once per build, and the warning clears itself
// the moment the file is dropped in.
function shippedFace(): Plugin {
  const at = (name: string) => fileURLToPath(new URL(`./src/fonts/${name}`, import.meta.url));
  return {
    name: "sprawling:shipped-face",
    apply: "build",
    generateBundle() {
      for (const name of [SANS, MONO, LICENCE]) {
        const path = at(name);
        if (!existsSync(path)) {
          this.warn(
            name === LICENCE
              ? `client/src/fonts/${LICENCE} is absent; a font may not be shipped without it`
              : `client/src/fonts/${name} is absent; the page falls back to the system stack`,
          );
          continue;
        }
        if (name === LICENCE) {
          this.emitFile({ type: "asset", fileName: `fonts/${LICENCE}`, source: readFileSync(path) });
        }
      }
    },
  };
}

// The bundle lands where `crates/sprawling/build.rs` reads it: the
// directory named `web-dist` under the workspace's `target/`, with
// `index.html` at its root. `base: './'` keeps every asset reference
// relative, so the same bundle serves from inside the binary.
export default defineConfig({
  root: "src",
  base: "./",
  plugins: [tailwindcss(), solid(), shippedFace()],
  build: {
    outDir: "../../target/web-dist",
    emptyOutDir: true,
  },
});
