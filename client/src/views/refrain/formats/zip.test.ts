// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { describe, expect, test } from "bun:test";

import { deflated, utf8, zipOf } from "../../gallery/fmt_samples";
import { UNPACKED_MAX, unpack } from "./zip";

describe("unpack", () => {
  test("reads stored and deflated parts as the directory states them", async () => {
    const text = utf8("<w:document>".repeat(200));
    const archive = zipOf([
      { name: "a.txt", bytes: utf8("stored") },
      { name: "word/document.xml", bytes: text, packed: await deflated(text) },
    ]);
    expect(await unpack(archive)).toEqual({
      kind: "unpacked",
      parts: new Map([
        ["a.txt", utf8("stored")],
        ["word/document.xml", text],
      ]),
    });
  });

  test("refuses unopened an archive that promises more than a page holds", async () => {
    const archive = zipOf([{ name: "big.bin", bytes: utf8("x"), packed: await deflated(utf8("x")), stated: UNPACKED_MAX + 1 }]);
    expect(await unpack(archive)).toEqual({ kind: "refused", why: "too_large" });
  });

  test("stops a part that inflates past the size it promised", async () => {
    const bomb = new Uint8Array(1024 * 1024);
    const archive = zipOf([{ name: "word/document.xml", bytes: bomb, packed: await deflated(bomb), stated: 1024 }]);
    expect(await unpack(archive)).toEqual({ kind: "refused", why: "shape" });
  });

  test("refuses bytes that are not an archive, and an archive cut short", async () => {
    const archive = zipOf([{ name: "a.txt", bytes: utf8("stored") }]);
    expect(await unpack(utf8("%PDF-1.7 not a zip"))).toEqual({ kind: "refused", why: "shape" });
    expect(await unpack(archive.subarray(10))).toEqual({ kind: "refused", why: "shape" });
  });

  test("refuses deflated bytes that do not inflate", async () => {
    const archive = zipOf([{ name: "a.txt", bytes: utf8("stored"), packed: utf8("\u00ff\u00ff not deflate") }]);
    expect(await unpack(archive)).toEqual({ kind: "refused", why: "shape" });
  });
});
