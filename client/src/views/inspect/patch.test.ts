// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// A patch's rows as the seat hands them to any look (client D95): the
// number a code row carries, and the quote bag that is there only where
// the page has a conversation (client/Spec.lean §7-2). No look is
// imported here.

import { describe, expect, test } from "bun:test";

import { GitOid } from "../../wire";
import type { HunksAnswer } from "../../wire";
import type { CodeLine } from "../changes";
import { lookOf, type PatchHands, type PatchRow } from "./patch";

const PATCH: HunksAnswer = {
  oid_a: GitOid.make("7c41e09aa1b2c3d4e5f60718293a4b5c6d7e8f90"),
  oid_b: GitOid.make("a1b2c3d4e5f60718293a4b5c6d7e8f9012345678"),
  path: "crates/city/src/document.rs",
  lines: ["@@ -38,2 +38,2 @@", " keep", "-gone", "+came"].map((text, at) => ({ number: at + 1, text })),
  withheld: [],
};

const WORDS = {
  withheld: (n: string, reason: string) => `${n}: ${reason}`,
  folded: (n: string) => `${n} folded`,
  quote: (n: string) => `quote line ${n}`,
};

const NOWHERE: PatchHands = { href: undefined, choose: () => undefined };

type CodeRow = Extract<PatchRow, { readonly kind: "code" }>;

function codeRows(rows: readonly PatchRow[]): readonly CodeRow[] {
  return rows.flatMap((row) => (row.kind === "code" ? [row] : []));
}

describe("a code row carries the one number it has in its own tree", () => {
  test("kept and added lines read the new tree, a removed line the old one", () => {
    const code = codeRows(lookOf(PATCH, null, WORDS, NOWHERE).rows).map((row) => [row.tone, row.place, row.sign]);
    expect(code).toEqual([
      ["context", "38", ""],
      ["removed", "39", "\u2212"],
      ["added", "39", "+"],
    ]);
  });
});

describe("a number quotes its line only where the page has a conversation", () => {
  test("no conversation, no quote bag", () => {
    expect(codeRows(lookOf(PATCH, null, WORDS, NOWHERE).rows).map((row) => row.quote)).toEqual([undefined, undefined, undefined]);
  });

  test("the bag links to the conversation, names the line and hands the chosen line back", () => {
    const chosen: CodeLine[] = [];
    const hands: PatchHands = { href: "#/talk/release/ledger", choose: (line) => chosen.push(line) };
    const removed = codeRows(lookOf(PATCH, 3, WORDS, hands).rows).find((row) => row.tone === "removed");
    expect(removed?.chosen).toBe(true);
    expect(removed?.quote?.href).toBe("#/talk/release/ledger");
    expect(removed?.quote?.["aria-label"]).toBe("quote line 39");
    removed?.quote?.onclick();
    expect(chosen.map((line) => line.number)).toEqual([3]);
  });
});
