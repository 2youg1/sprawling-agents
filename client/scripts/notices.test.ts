// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { describe, expect, test } from "bun:test";

import { noticesText, packagesIn } from "./notices";

describe("third-party notices", () => {
  test("each installed package is named once, scoped and nested ones by their own name", () => {
    expect(
      packagesIn([
        "/w/client/node_modules/svelte/src/internal/client/index.js",
        "/w/client/node_modules/@lezer/rust/dist/index.js",
        "/w/client/node_modules/@lezer/rust/dist/other.js",
        "/w/client/node_modules/a/node_modules/b/x.js",
        "C:\\w\\client\\node_modules\\effect\\dist\\esm\\Schema.js",
        "\0/w/client/node_modules/svelte/src/index-client.js",
        "/w/client/src/main.ts",
      ]),
    ).toEqual([
      { name: "@lezer/rust", dir: "/w/client/node_modules/@lezer/rust" },
      { name: "b", dir: "/w/client/node_modules/a/node_modules/b" },
      { name: "effect", dir: "C:/w/client/node_modules/effect" },
      { name: "svelte", dir: "/w/client/node_modules/svelte" },
    ]);
  });

  test("the file states each package with its licence texts verbatim, line endings folded", () => {
    expect(
      noticesText([
        { name: "effect", version: "3.22.1", license: "MIT", texts: ["MIT License\r\n\r\nCopyright (c) E\r\n"] },
        { name: "svelte", version: "5.57.1", license: "MIT", texts: ["Copyright (c) S\n", "NOTICE text\n"] },
      ]),
    ).toBe(
      [
        "The npm packages this bundle was built from, each with the licence text it ships.",
        "The licence of the fonts is fonts/OFL.txt.",
        "",
        "-".repeat(72),
        "",
        "effect 3.22.1 (MIT)",
        "",
        "MIT License",
        "",
        "Copyright (c) E",
        "",
        "-".repeat(72),
        "",
        "svelte 5.57.1 (MIT)",
        "",
        "Copyright (c) S",
        "",
        "NOTICE text",
        "",
      ].join("\n"),
    );
  });
});
