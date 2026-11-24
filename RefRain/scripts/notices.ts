// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { existsSync, readdirSync, readFileSync } from "node:fs";
import { join, resolve } from "node:path";

import type { Plugin } from "vite";

export interface Installed {
  readonly name: string;
  readonly dir: string;
}

export interface Notice {
  readonly name: string;
  readonly version: string;
  readonly license: string;
  readonly texts: readonly string[];
}

const FILE = "THIRD-PARTY-NOTICES.txt";

const MODULES = "node_modules/";

const LICENCE_FILE = /^(licen[cs]e|copying|notice)/i;

const SEPARATOR = "-".repeat(72);

function head(licences: readonly string[]): readonly string[] {
  const where =
    licences.length === 0
      ? "The fonts this bundle ships carry their licences under fonts/."
      : `The fonts this bundle ships carry theirs as ${licences
          .map((name) => `fonts/${name}`)
          .join(", ")}.`;
  return [
    "The npm packages this bundle was built from, each with the licence text it ships.",
    where,
  ];
}

export function thirdPartyNotices(licences: readonly string[]): Plugin {
  let root = "";
  return {
    name: "sprawling:third-party-notices",
    apply: "build",
    configResolved(config) {
      root = config.root;
    },
    generateBundle(_options, bundle) {
      const moduleIds = Object.values(bundle).flatMap((item) =>
        item.type === "chunk" ? item.moduleIds : [],
      );
      const assets = Object.values(bundle).flatMap((item) =>
        item.type === "asset"
          ? item.originalFileNames.map((name) => resolve(root, name).replaceAll("\\", "/"))
          : [],
      );
      const read = packagesIn([...moduleIds, ...assets]).map((installed) => ({
        installed,
        notice: noticeOf(installed, foldersOf(installed, assets)),
      }));
      const unlicensed = read.filter((entry) => entry.notice === undefined);
      if (unlicensed.length > 0) {
        this.error(
          `these packages reach the bundle without a licence file: ${unlicensed
            .map((entry) => `${entry.installed.name} (${entry.installed.dir})`)
            .join(", ")}`,
        );
      } else {
        const notices = read.flatMap((entry) => (entry.notice === undefined ? [] : [entry.notice]));
        notices.push({
          name: "Tailwind CSS reset and utility selectors",
          version: "4",
          license: "MIT",
          texts: [readFileSync(new URL("../licenses/Tailwind-MIT.txt", import.meta.url), "utf8")],
        });
        this.emitFile({ type: "asset", fileName: FILE, source: noticesText(notices, licences) });
      }
    },
  };
}

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

    if (name === undefined || name === "" || name.endsWith("/")) {
      continue;
    }
    const dir = `${base}${name}`;
    found.set(dir, { name, dir });
  }
  return [...found.values()].sort((a, b) => order(a.name, b.name) || order(a.dir, b.dir));
}

export function noticesText(entries: readonly Notice[], licences: readonly string[]): string {
  const blocks = entries.flatMap((entry) => [
    SEPARATOR,
    [`${entry.name} ${entry.version} (${entry.license})`, ...entry.texts.map(folded)].join("\n\n"),
  ]);
  return `${[head(licences).join("\n"), ...blocks].join("\n\n")}\n`;
}
export function foldersOf(installed: Installed, assets: readonly string[]): readonly string[] {
  const root = `${installed.dir}/`;
  const folders = assets.flatMap((path) => {
    const inside = path.startsWith(root) ? path.slice(root.length) : "";
    const cut = inside.lastIndexOf("/");
    return cut <= 0 ? [] : [inside.slice(0, cut)];
  });
  return [...new Set(folders)].sort(order);
}
function noticeOf(installed: Installed, folders: readonly string[]): Notice | undefined {
  const manifest = join(installed.dir, "package.json");
  if (!existsSync(manifest)) {
    return undefined;
  }
  const stated: unknown = JSON.parse(readFileSync(manifest, "utf8"));
  const version = field(stated, "version");
  let license = field(stated, "license");
  const own = licencesIn(installed.dir);
  if (
    license === "unstated" &&
    installed.name === "khroma" &&
    version === "2.1.0" &&
    own.includes(readFileSync(new URL("../licenses/khroma-2.1.0-MIT.txt", import.meta.url), "utf8"))
  ) {
    license = "MIT";
  }
  if (
    own.length === 0 &&
    version === "2.0.3" &&
    license === "MIT" &&
    ["solid-js", "@solidjs/web", "@solidjs/signals"].includes(installed.name)
  ) {
    own.push(readFileSync(new URL("../licenses/Solid-2.0.3-MIT.txt", import.meta.url), "utf8"));
  }
  if (
    own.length === 0 &&
    installed.name === "fastdom" &&
    version === "1.0.12" &&
    license === "MIT"
  ) {
    const readme = readFileSync(join(installed.dir, "README.md"), "utf8");
    const section = /^## License\r?\n([\s\S]*?)(?:\r?\n## |(?![\s\S]))/m.exec(readme);
    const terms = section?.[1]?.trim();
    if (terms?.includes("THE SOFTWARE IS PROVIDED")) own.push(terms);
  }
  if (own.length === 0) return undefined;
  const texts = [...own, ...folders.flatMap((folder) => licencesIn(join(installed.dir, folder)))];
  return {
    name: installed.name,
    version,
    license,
    texts,
  };
}

function licencesIn(folder: string): string[] {
  return readdirSync(folder, { withFileTypes: true })
    .filter((entry) => entry.isFile() && LICENCE_FILE.test(entry.name))
    .map((entry) => entry.name)
    .sort(order)
    .map((name) => readFileSync(join(folder, name), "utf8"));
}

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

function order(a: string, b: string): number {
  return a < b ? -1 : a > b ? 1 : 0;
}
