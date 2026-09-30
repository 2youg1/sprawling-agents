// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The notices of the npm packages inside the bundle (client-SPEC 12-13).
// MIT and Apache-2.0 both require the copyright and licence notice to
// travel with every copy, and a minified bundle is a copy. The packages
// are read off the chunks the bundler emitted rather than off
// `package.json` or `bun.lock`: the manifest misses what a runtime
// package brings in, and the lockfile counts the toolchain and every
// module tree-shaking dropped.

import { existsSync, readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";

import type { Plugin } from "vite";

/** One npm package the bundle was built from, and the directory it was read from. */
export interface Installed {
  readonly name: string;
  readonly dir: string;
}

/** What the notices file says about one package. */
export interface Notice {
  readonly name: string;
  readonly version: string;
  readonly license: string;
  readonly texts: readonly string[];
}

/** The asset's name, at the root of the bundle the binary embeds and serves. */
const FILE = "THIRD-PARTY-NOTICES.txt";

const MODULES = "node_modules/";

/** A file at a package's top level that carries its licence or notice. */
const LICENCE_FILE = /^(licen[cs]e|copying|notice)/i;

const SEPARATOR = "-".repeat(72);

const HEAD = [
  "The npm packages this bundle was built from, each with the licence text it ships.",
  "The licence of the fonts is fonts/OFL.txt.",
];

// A package that reaches the bundle without a licence file stops the
// build and is named: a notices file that silently lacks one package is
// the gap it exists to close.
export function thirdPartyNotices(): Plugin {
  return {
    name: "sprawling:third-party-notices",
    apply: "build",
    generateBundle(_options, bundle) {
      const moduleIds = Object.values(bundle).flatMap((item) =>
        item.type === "chunk" ? item.moduleIds : [],
      );
      const read = packagesIn(moduleIds).map((installed) => ({ installed, notice: noticeOf(installed) }));
      const unlicensed = read.filter((entry) => entry.notice === undefined);
      if (unlicensed.length > 0) {
        this.error(
          `these packages reach the bundle without a licence file: ${unlicensed
            .map((entry) => `${entry.installed.name} (${entry.installed.dir})`)
            .join(", ")}`,
        );
      } else {
        const notices = read.flatMap((entry) => (entry.notice === undefined ? [] : [entry.notice]));
        this.emitFile({ type: "asset", fileName: FILE, source: noticesText(notices) });
      }
    },
  };
}

/**
 * The packages a set of module ids was read from: the path after the last
 * `node_modules/`, one segment, or two when the first is a scope. Each
 * installed directory appears once, sorted by name and then directory, so
 * the result depends on the ids alone and not on their order.
 */
export function packagesIn(moduleIds: Iterable<string>): readonly Installed[] {
  const found = new Map<string, Installed>();
  for (const id of moduleIds) {
    const path = id.replaceAll("\\", "/").replace(/^\0+/, "");
    const at = path.lastIndexOf(MODULES);
    if (at < 0) {
      continue;
    }
    const base = path.slice(0, at + MODULES.length);
    const [first, second] = path.slice(at + MODULES.length).split("/");
    const scoped = first?.startsWith("@") === true;
    const name = scoped ? (second === undefined ? undefined : `${first}/${second}`) : first;
    // A bare scope with nothing after it names no package.
    if (name === undefined || name === "" || name.endsWith("/")) {
      continue;
    }
    const dir = `${base}${name}`;
    found.set(dir, { name, dir });
  }
  return [...found.values()].sort((a, b) => order(a.name, b.name) || order(a.dir, b.dir));
}

/**
 * The whole notices file: two lines that say what it is, then each
 * package's name, version and declared licence followed by its licence
 * files verbatim, line endings folded to `\n`.
 */
export function noticesText(entries: readonly Notice[]): string {
  const blocks = entries.flatMap((entry) => [
    SEPARATOR,
    [`${entry.name} ${entry.version} (${entry.license})`, ...entry.texts.map(folded)].join("\n\n"),
  ]);
  return `${[HEAD.join("\n"), ...blocks].join("\n\n")}\n`;
}

/** What one installed package says about itself, or nothing when it ships no licence file. */
function noticeOf(installed: Installed): Notice | undefined {
  const manifest = join(installed.dir, "package.json");
  if (!existsSync(manifest)) {
    return undefined;
  }
  const texts = readdirSync(installed.dir, { withFileTypes: true })
    .filter((entry) => entry.isFile() && LICENCE_FILE.test(entry.name))
    .map((entry) => entry.name)
    .sort(order)
    .map((name) => readFileSync(join(installed.dir, name), "utf8"));
  if (texts.length === 0) {
    return undefined;
  }
  const stated: unknown = JSON.parse(readFileSync(manifest, "utf8"));
  return {
    name: installed.name,
    version: field(stated, "version"),
    license: field(stated, "license"),
    texts,
  };
}

/** A string field of a parsed `package.json`, or `unstated`. */
function field(stated: unknown, key: "version" | "license"): string {
  if (typeof stated !== "object" || stated === null || !(key in stated)) {
    return "unstated";
  }
  const value: unknown = Reflect.get(stated, key);
  return typeof value === "string" ? value : "unstated";
}

function folded(text: string): string {
  return text.replaceAll("\r\n", "\n").trimEnd();
}

/** Code-unit order, so the file's bytes do not depend on the machine's locale. */
function order(a: string, b: string): number {
  return a < b ? -1 : a > b ? 1 : 0;
}
