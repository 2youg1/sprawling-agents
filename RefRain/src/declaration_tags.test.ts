// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { describe, expect, test } from "bun:test";
import { readFileSync, readdirSync } from "node:fs";
import { join, relative } from "node:path";

const DECLARATION_TAG = /^\s*\{(?:const|let)\s/;

function svelteFiles(dir: string): readonly string[] {
  return readdirSync(dir, { withFileTypes: true }).flatMap((entry) => {
    const path = join(dir, entry.name);
    if (entry.isDirectory()) return svelteFiles(path);
    return entry.name.endsWith(".svelte") ? [path] : [];
  });
}

describe("declaration tags", () => {
  test("no template declares a value with a plain {const} or {let} tag", () => {
    const src = import.meta.dirname;
    const found = svelteFiles(src).flatMap((file) =>
      readFileSync(file, "utf8")
        .split("\n")
        .flatMap((line, index) =>
          DECLARATION_TAG.test(line) ? [`${relative(src, file)}:${String(index + 1)}`] : [],
        ),
    );
    expect(found).toEqual([]);
  });
});
