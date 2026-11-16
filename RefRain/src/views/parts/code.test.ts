// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The inks follow the grammar, not the characters: a Rust lifetime opens
// with the same quote a character literal does, and only a parser can
// tell the two apart. A reader that guesses from characters paints the
// rest of the line as a string, which hides the code it was asked to
// show.

import { describe, expect, test } from "bun:test";

import { painted, type Piece } from "./code";

// The pieces that carry an ink, in order: plain text between them is
// the part a grammar has no opinion about.
function inked(pieces: readonly Piece[]): readonly Piece[] {
  return pieces.filter((piece) => piece.ink !== "plain");
}

describe("code is coloured by its grammar", () => {
  test("a lifetime is not a string, and the text survives whole", async () => {
    const line = "fn keep<'a>(held: &'a str) -> u8 { 1 }";
    const pieces = await painted(line, "src/keep.rs");
    expect(pieces.map((piece) => piece.text).join("")).toBe(line);
    expect(inked(pieces)).toEqual([
      { ink: "word", text: "fn" },
      { ink: "number", text: "1" },
    ]);
  });

  test("a Markdown fence names its grammar by word", async () => {
    const pieces = await painted("held = 'one'  # kept", "python");
    expect(inked(pieces)).toEqual([
      { ink: "string", text: "'one'" },
      { ink: "comment", text: "# kept" },
    ]);
  });
});
