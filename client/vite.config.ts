// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { existsSync, readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

import tailwindcss from "@tailwindcss/vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";
import { defineConfig } from "vite";
import type { Plugin } from "vite";

import { thirdPartyNotices } from "./scripts/notices.ts";

// The files the shipped faces are made of are the ones `theme/fonts.css`
// names, and their licences are the `.txt` files beside them: two lists
// that already exist in the tree, so adding a face adds its file and its
// licence and this plugin needs no edit. OFL-1.1 requires each licence to
// travel with its font, so every one of them is put in the bundle beside
// the fonts, and a face whose licence is missing is named here rather
// than shipped without one. A face's licence is named after the family it
// belongs to — `PaperMono-OFL.txt` for `Paper Mono` — which is the one
// rule this plugin states.
const FONTS = fileURLToPath(new URL("./src/theme/fonts.css", import.meta.url));
const FACES_DIR = fileURLToPath(new URL("./src/fonts/", import.meta.url));

/// The letters a name's words are made of, so `Paper Mono` and
/// `PaperMono-OFL.txt` compare equal.
function letters(text: string): string {
  return text.replaceAll(/[^\p{L}\p{N}]/gu, "").toLowerCase();
}

/// Every source a `@font-face` names, and every family one is declared
/// under, read out of the stylesheet that owns both.
function declaredFaces(): {
  readonly files: readonly string[];
  readonly families: readonly string[];
} {
  const css = readFileSync(FONTS, "utf8");
  const files = [...css.matchAll(/url\(\s*"?\.\.\/fonts\/([^"')]+)"?\s*\)/g)].flatMap((match) =>
    match[1] === undefined ? [] : [match[1]],
  );
  const families = [...css.matchAll(/font-family:\s*"([^"]+)"/g)].flatMap((match) =>
    match[1] === undefined ? [] : [match[1]],
  );
  return { files: [...new Set(files)], families: [...new Set(families)] };
}

/// The licence files beside the fonts, sorted: every `.txt` in the
/// directory, because a text file next to a font is that font's licence.
/// A tree without the directory is a bundle that draws with what the
/// machine has rather than a failed build, so it answers with none.
function declaredLicences(): readonly string[] {
  if (!existsSync(FACES_DIR)) return [];
  return readdirSync(FACES_DIR)
    .filter((name) => name.endsWith(".txt"))
    .sort();
}

// A face that is not in the tree is not a broken build: the fallback
// chain draws the page with what the machine has. It is still a bundle
// nobody should ship without noticing, so each missing file is named
// once per build, and the warning clears itself the moment the file is
// dropped in.
function shippedFace(): Plugin {
  const at = (name: string) => join(FACES_DIR, name);
  const licences = declaredLicences();
  return {
    name: "sprawling:shipped-face",
    apply: "build",
    generateBundle() {
      const { files, families } = declaredFaces();
      for (const name of files) {
        if (!existsSync(at(name))) {
          this.warn(`client/src/fonts/${name} is absent; the page falls back to the system stack`);
        }
      }
      for (const family of families) {
        const named = licences.some((name) => letters(name).startsWith(letters(family)));
        if (!named) {
          this.warn(
            `client/src/fonts/ holds no licence for ${family}; a font may not be shipped without one`,
          );
        }
      }
      for (const name of licences) {
        this.emitFile({ type: "asset", fileName: `fonts/${name}`, source: readFileSync(at(name)) });
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
    thirdPartyNotices(declaredLicences()),
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
