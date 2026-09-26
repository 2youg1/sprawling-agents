// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { describe, expect, test } from "bun:test";

import { editorLink } from "./editor";

describe("editorLink", () => {
  test("spells the documented file:line:column form for a path inside the city", () => {
    expect([
      editorLink({ editor: "vscode", folder: ["D:", "cities", "my city", ""].join("\\"), path: "shop/src/main.rs", line: 42 }),
      editorLink({ editor: "vscode-insiders", folder: "/srv/city", path: "shop/a b.rs", line: 1 }),
      editorLink({ editor: "none", folder: "/srv/city", path: "shop/main.rs", line: 1 }),
    ]).toEqual([
      "vscode://file/D:/cities/my%20city/shop/src/main.rs:42:1",
      "vscode-insiders://file/srv/city/shop/a%20b.rs:1:1",
      null,
    ]);
  });

  test("offers no link for anything outside the city or not a line", () => {
    const outside = [
      { path: "../secret.txt", folder: "/srv/city", line: 3 },
      { path: "shop/../../etc/passwd", folder: "/srv/city", line: 3 },
      { path: "C:/Windows/win.ini", folder: "/srv/city", line: 3 },
      { path: "/etc/passwd", folder: "/srv/city", line: 3 },
      { path: String.raw`shop\..\x`, folder: "/srv/city", line: 3 },
      { path: "shop/main.rs", folder: "relative/city", line: 3 },
      { path: "shop/main.rs", folder: "/srv/../b", line: 3 },
      { path: "shop/main.rs", folder: "", line: 3 },
      { path: "shop/main.rs", folder: "/srv/city", line: 0 },
      { path: "shop/main.rs", folder: "/srv/city", line: 1.5 },
    ];
    expect(outside.map((each) => editorLink({ editor: "vscode", ...each }))).toEqual(outside.map(() => null));
  });
});
