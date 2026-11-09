// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The fallback chain as the stylesheet reads it. Two halves meet here and
// neither can be checked by reading one: the page writes the names, each
// with the comma after it, and the family stacks put that token where a
// family would stand. A chain of one, of two, and of none are the three
// cases the stacks have to stay valid through.

import { expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

import { faceStyle, fallbackToken } from "./appearance";

const theme = (name: string): string =>
  readFileSync(fileURLToPath(new URL(`../../theme/${name}`, import.meta.url)), "utf8");

test("each name carries the comma that joins it to the next, and none is a chain of none", () => {
  expect(fallbackToken([])).toBe("");
  expect(fallbackToken(["Maple Mono NF CN"])).toBe('"Maple Mono NF CN",');
  expect(fallbackToken(["Maple Mono NF CN", "Noto Sans SC"])).toBe(
    '"Maple Mono NF CN", "Noto Sans SC",',
  );
});

test("a picker row quotes the family and keeps a generic behind it", () => {
  expect(faceStyle("Iosevka")).toBe('"Iosevka", ui-sans-serif, system-ui, sans-serif');
  expect(faceStyle('Font "Display"')).toBe(
    '"Font \\"Display\\"", ui-sans-serif, system-ui, sans-serif',
  );
});

test("the stacks read the token where a family stands, with the comma of their own for an unnamed chain", () => {
  const stacks = theme("tokens-type.css");
  for (const [role, face] of [
    ["--font-sans", "--face-sans"],
    ["--font-mono", "--face-mono"],
  ] as const) {
    expect(stacks, `${role} carries the chain`).toContain(
      `${role}:\n    var(${face}), var(--face-fallback,)`,
    );
  }
  expect(theme("preference.css")).toContain('"Libron", var(--face-fallback,) serif');
});
