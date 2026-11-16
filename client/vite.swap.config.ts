// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { readdirSync } from "node:fs";
import { join, relative } from "node:path";

import { mergeConfig, normalizePath } from "vite";
import type { Plugin, UserConfig } from "vite";

import shipped from "./vite.config";

const SOURCE = normalizePath(join(import.meta.dirname, "src"));
const SWAP = normalizePath(join(import.meta.dirname, "swap"));
const LOOK = ".look.svelte";

const OUT = "../../target/swap-web";

function replacements(dir: string): string[] {
  return readdirSync(dir, { recursive: true, encoding: "utf8" })
    .map((each) => normalizePath(each))
    .filter((each) => each.endsWith(LOOK));
}

function swapLooks(): Plugin {
  const wanted = new Set(replacements(SWAP));
  const swapped = new Set<string>();
  return {
    name: "sprawling:swap-looks",
    enforce: "pre",
    async resolveId(source, importer, options) {
      if (importer === undefined || !source.endsWith(LOOK)) return null;
      const resolved = await this.resolve(source, importer, { ...options, skipSelf: true });
      if (resolved === null) return null;
      const id = normalizePath(resolved.id);
      if (!id.startsWith(`${SOURCE}/`)) return null;
      const path = normalizePath(relative(SOURCE, id));
      if (!wanted.has(path)) return null;
      swapped.add(path);
      return `${SWAP}/${path}`;
    },
    buildEnd(failure) {
      if (failure !== undefined) return;
      if (wanted.size === 0) {
        this.error(`swap/ holds no ${LOOK} file, so this build would replace nothing`);
      }
      for (const path of wanted) {
        if (!swapped.has(path)) {
          this.error(`swap/${path} replaced nothing: no seat in the bundle imports src/${path}`);
        }
      }
      this.info(`replaced ${String(swapped.size)} look(s): ${[...swapped].join(", ")}`);
    },
  };
}

const swap: UserConfig = {
  plugins: [swapLooks()],
  build: { outDir: OUT, emptyOutDir: true },
};

export default mergeConfig(shipped, swap);
