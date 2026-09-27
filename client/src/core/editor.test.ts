// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { describe, expect, test } from "bun:test";

import { EDITORS, editorLink } from "./editor";

describe("editorLink", () => {
  test("spells each editor's file:line:column form for a path inside the city", () => {
    const windows = ["D:", "cities", "my city", ""].join("\\");
    expect([
      editorLink({ editor: "vscode", folder: windows, path: "shop/src/main.rs", line: 42 }),
      editorLink({ editor: "vscode-insiders", folder: "/srv/city", path: "shop/a b.rs", line: 1 }),
      editorLink({ editor: "vscodium", folder: "/srv/city", path: "shop/main.rs", line: 7 }),
      editorLink({ editor: "cursor", folder: windows, path: "shop/main.rs", line: 7 }),
      editorLink({ editor: "windsurf", folder: "/srv/city", path: "shop/main.rs", line: 7 }),
      editorLink({ editor: "zed", folder: "/srv/city", path: "shop/a b.rs", line: 7 }),
      editorLink({ editor: "zed", folder: windows, path: "shop/main.rs", line: 7 }),
      editorLink({ editor: "none", folder: "/srv/city", path: "shop/main.rs", line: 1 }),
    ]).toEqual([
      "vscode://file/D:/cities/my%20city/shop/src/main.rs:42:1",
      "vscode-insiders://file/srv/city/shop/a%20b.rs:1:1",
      "vscodium://file/srv/city/shop/main.rs:7:1",
      "cursor://file/D:/cities/my%20city/shop/main.rs:7:1",
      "windsurf://file/srv/city/shop/main.rs:7:1",
      "zed://file/srv/city/shop/a%20b.rs:7:1",
      "zed://file//%3F/D:/cities/my%20city/shop/main.rs:7:1",
      null,
    ]);
  });

  // A browser hands the link on in the form its URL parser writes back,
  // so a link that parser rewrites is not the link the editor receives.
  test("survives the browser's URL parser unchanged", () => {
    const links = EDITORS.map((editor) =>
      editorLink({ editor, folder: String.raw`D:\cities\a b`, path: "shop/main.rs", line: 3 }),
    ).filter((link) => link !== null);
    expect(links.map((link) => new URL(link).href)).toEqual(links);
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
      { path: "shop/main.rs", folder: String.raw`\\server\share\city`, line: 3 },
      { path: "shop/main.rs", folder: "//server/share/city", line: 3 },
      { path: "shop/main.rs", folder: "/srv/city", line: 0 },
      { path: "shop/main.rs", folder: "/srv/city", line: 1.5 },
    ];
    expect(outside.map((each) => editorLink({ editor: "vscode", ...each }))).toEqual(outside.map(() => null));
  });
});
