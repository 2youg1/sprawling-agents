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

const FONTS = fileURLToPath(new URL("./src/theme/fonts.css", import.meta.url));
const FACES_DIR = fileURLToPath(new URL("./src/fonts/", import.meta.url));

function letters(text: string): string {
  return text.replaceAll(/[^\p{L}\p{N}]/gu, "").toLowerCase();
}

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

function declaredLicences(): readonly string[] {
  if (!existsSync(FACES_DIR)) return [];
  return readdirSync(FACES_DIR)
    .filter((name) => name.endsWith(".txt"))
    .sort();
}

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
          this.warn(`RefRain/src/fonts/${name} is absent; the page falls back to the system stack`);
        }
      }
      for (const family of families) {
        const named = licences.some((name) => letters(name).startsWith(letters(family)));
        if (!named) {
          this.warn(
            `RefRain/src/fonts/ holds no licence for ${family}; a font may not be shipped without one`,
          );
        }
      }
      for (const name of licences) {
        this.emitFile({ type: "asset", fileName: `fonts/${name}`, source: readFileSync(at(name)) });
      }
    },
  };
}

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
    outDir: "../../Sprawling/app/web-dist",
    emptyOutDir: true,

    sourcemap: false,
  },
});
