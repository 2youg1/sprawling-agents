// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { describe, expect, test } from "bun:test";

import { GitOid, type HunksAnswer } from "../wire";
import { numbered } from "./changes";

const OID_A = GitOid.make("a".repeat(40));
const OID_B = GitOid.make("b".repeat(40));

function patch(texts: readonly string[], withheld: HunksAnswer["withheld"] = []): HunksAnswer {
  const held = new Set(withheld.map((each) => each.number));
  const lines = texts
    .map((text, at) => ({ number: at + 1, text }))
    .filter((line) => !held.has(line.number));
  return { lines, oid_a: OID_A, oid_b: OID_B, path: "src/lib.rs", withheld };
}

describe("numbered", () => {
  test("numbers both files from the hunk headers and folds what git left out", () => {
    const drawn = numbered(
      patch([
        "diff --git a/src/lib.rs b/src/lib.rs",
        "--- a/src/lib.rs",
        "+++ b/src/lib.rs",
        "@@ -10,2 +10,2 @@ fn main() {",
        " keep",
        "-gone",
        "+come",
        "@@ -40,1 +40,2 @@",
        " tail",
        "+more",
      ]),
    );
    expect(drawn).toEqual([
      { kind: "head", number: 1, text: "diff --git a/src/lib.rs b/src/lib.rs" },
      { kind: "head", number: 2, text: "--- a/src/lib.rs" },
      { kind: "head", number: 3, text: "+++ b/src/lib.rs" },
      { kind: "hunk", number: 4, text: "@@ -10,2 +10,2 @@ fn main() {", folded: 9 },
      { kind: "context", number: 5, text: "keep", old: 10, new: 10 },
      { kind: "removed", number: 6, text: "gone", old: 11 },
      { kind: "added", number: 7, text: "come", new: 11 },
      { kind: "hunk", number: 8, text: "@@ -40,1 +40,2 @@", folded: 28 },
      { kind: "context", number: 9, text: "tail", old: 40, new: 40 },
      { kind: "added", number: 10, text: "more", new: 41 },
    ]);
  });

  test("a withheld line stands in its place and leaves the count unknown until the next hunk", () => {
    const drawn = numbered(
      patch(["@@ -1,2 +1,2 @@", "+key = hidden", " after", "@@ -9 +9 @@", " known"], [
        { number: 2, reason: "secret" },
      ]),
    );
    expect(drawn).toEqual([
      { kind: "hunk", number: 1, text: "@@ -1,2 +1,2 @@", folded: 0 },
      { kind: "withheld", number: 2, reason: "secret" },
      { kind: "context", number: 3, text: "after", old: null, new: null },
      { kind: "hunk", number: 4, text: "@@ -9 +9 @@", folded: 6 },
      { kind: "context", number: 5, text: "known", old: 9, new: 9 },
    ]);
  });
});
