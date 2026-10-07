// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { readdirSync } from "node:fs";
import { join, relative } from "node:path";

import { mergeConfig, normalizePath } from "vite";
import type { Plugin, UserConfig } from "vite";

import shipped from "./vite.config";

// The swap build (tools/xtask/Spec.lean §8-13, `just swap`): the shipped
// client with every look under `swap/` drawn in place of the look at the
// same path under `src/`. A seat imports its look as `./x.look.svelte`,
// so the replacement is matched on the path the import resolves to, not
// on the specifier a caller wrote.
const SOURCE = normalizePath(join(import.meta.dirname, "src"));
const SWAP = normalizePath(join(import.meta.dirname, "swap"));
const LOOK = ".look.svelte";

// Outside `crates/sprawling/web-dist`, which the binary embeds: a swap
// build stopped half-way must not leave a replaced bundle for the next
// `cargo build` to ship. Under the repository's own `target/` rather
// than `CARGO_TARGET_DIR`, which worktrees may share.
const OUT = "../../target/swap-web";

/** Every replacement look, as a path relative to `swap/`. */
function replacements(dir: string): string[] {
  return readdirSync(dir, { recursive: true, encoding: "utf8" })
    .map((each) => normalizePath(each))
    .filter((each) => each.endsWith(LOOK));
}

// A swap build that replaced nothing is the shipped bundle under another
// name, and would pass every check while proving nothing; a replacement
// with no look to stand in for is a file nobody's seat will ever reach.
// Both refuse the build, and so does a replacement whose look exists but
// that no seat in the bundle imported.
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
