// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { describe, expect, test } from "bun:test";

import { B3Hash } from "../wire";
import { positionsOf, textEdits, utf16At } from "./document_pos";

const V = B3Hash.make("a".repeat(64));

describe("positionsOf", () => {
  test("counts a UTF-8 version's bytes per character and its columns per code point", () => {
    const at = positionsOf(V, "utf8", "héllo\nwörld");
    expect({
      version: at.version,
      editor: at.editor,
      lineBreak: at.lineBreak,
      bytes: [0, 1, 2, 6, 7, 8, 11].map((each) => at.bytes(each)),
      editorAt: [0, 1, 2, 3, 7, 8, 9, 13].map((each) => at.editorAt(each)),
      place: at.place(8),
    }).toEqual({
      version: V,
      editor: "héllo\nwörld",
      lineBreak: "\n",
      bytes: [0, 1, 3, 7, 8, 10, 13],
      editorAt: [0, 1, 1, 2, 6, 7, 7, 11],
      place: { line: 2, column: 3 },
    });
  });

  // A position inside a surrogate pair is not a character, and a byte
  // inside a four-byte character is not one either: both fall back to
  // the character's start rather than splitting it.
  test("falls back to a character's start from inside it", () => {
    const at = positionsOf(V, "utf8", "a😀b");
    expect([at.bytes(1), at.bytes(2), at.bytes(3), at.editorAt(3), at.editorAt(5), at.place(3)]).toEqual([
      1, 1, 5, 1, 3, { line: 1, column: 3 },
    ]);
  });

  test("folds the mark and every line break the way the editor reads them, and unfolds them in bytes", () => {
    const at = positionsOf(V, "utf8_bom", "\uFEFFa\r\nb\rc\n");
    expect({
      editor: at.editor,
      lineBreak: at.lineBreak,
      bytes: [0, 1, 2, 3, 4, 5, 6].map((each) => at.bytes(each)),
      editorAt: [3, 4, 5, 6, 7, 8, 9, 10].map((each) => at.editorAt(each)),
    }).toEqual({
      editor: "a\nb\nc\n",
      lineBreak: "\r\n",
      bytes: [3, 4, 6, 7, 8, 9, 10],
      editorAt: [0, 1, 1, 2, 3, 4, 5, 6],
    });
  });

  test("counts two bytes for each UTF-16 code unit", () => {
    const at = positionsOf(V, "utf16_le", "\uFEFFab\n😀");
    expect([at.editor, at.bytes(0), at.bytes(2), at.bytes(3), at.bytes(5), at.editorAt(12), at.editorAt(9)]).toEqual([
      "ab\n😀", 2, 6, 8, 12, 5, 3,
    ]);
  });

  // The checkpoints are a shortcut and must answer what a walk from the
  // first byte answers, on both sides of every one of them.
  test("answers a long version the way a walk from the start does", () => {
    const line = "réglé 😀 文字 plain\n";
    const text = line.repeat(400);
    const at = positionsOf(V, "utf8", text);
    const encoder = new TextEncoder();
    const probes = [0, 17, 1023, 1024, 1025, 2048, 4095, 5000, text.length];
    expect(probes.map((each) => at.bytes(each))).toEqual(
      probes.map((each) => encoder.encode(text.slice(0, each)).length),
    );
    expect(probes.map((each) => at.editorAt(at.bytes(each)))).toEqual(probes);
  });
});

describe("textEdits", () => {
  test("writes editor changes as the baseline's bytes, its line breaks and its mark left alone", () => {
    const at = positionsOf(V, "utf8_bom", "\uFEFFa\r\nb");
    expect(
      textEdits(at, [
        { from: 0, to: 0, insert: "x" },
        { from: 1, to: 2, insert: "" },
        { from: 2, to: 3, insert: "B\nC" },
      ]),
    ).toEqual([
      { span: { start: 3, end: 3 }, text: "x" },
      { span: { start: 4, end: 6 }, text: "" },
      { span: { start: 6, end: 7 }, text: "B\r\nC" },
    ]);
  });

  test("writes a new line as a newline in a version that has none", () => {
    const at = positionsOf(V, "utf8", "one line");
    expect(textEdits(at, [{ from: 8, to: 8, insert: "\nnext" }])).toEqual([
      { span: { start: 8, end: 8 }, text: "\nnext" },
    ]);
  });
});

// A reply's answer counts UTF-8 bytes of the stretch it was sent, and
// the page slices that stretch in UTF-16 code units.
describe("utf16At", () => {
  test("finds the code unit a byte stands at, falling back to a character's start and stopping at the end", () => {
    const text = "hé 文😀!";
    expect([0, 1, 2, 3, 4, 5, 7, 8, 11, 12, 13, 99].map((byte) => utf16At(text, byte))).toEqual([
      0, 1, 1, 2, 3, 3, 4, 4, 6, 7, 7, 7,
    ]);
  });
});
