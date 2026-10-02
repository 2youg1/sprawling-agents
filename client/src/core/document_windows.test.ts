// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { describe, expect, test } from "bun:test";

import { Address, B3Hash } from "../wire";
import type { DocumentAnswer, DocumentState } from "../wire";
import { EDITABLE_BYTES_MAX, joined, nextSpan, opened, whole } from "./document_windows";
import type { Gathering } from "./document_windows";

const AT = Address.make("shop/notes.md");
const V = B3Hash.make("a".repeat(64));
const W = B3Hash.make("b".repeat(64));

function answer(state: DocumentState): DocumentAnswer {
  return { at: AT, state };
}

function headOf(text: string, bytes: number): DocumentAnswer {
  return answer({
    held: {
      version: V,
      format: "markdown",
      bytes,
      body: { text: { encoding: "utf8", coverage: "head", head: { span: { start: 0, end: text.length }, text } } },
    },
  });
}

function gathering(read: ReturnType<typeof opened>): Gathering {
  if (read.kind !== "text") return { version: W, format: "plain", encoding: "utf8", bytes: -1, text: "", through: -1 };
  return read.gathering;
}

describe("opened", () => {
  test("tells the three ways a file has no text apart from an empty one", () => {
    expect([
      opened(answer("missing")),
      opened(answer({ unreadable: { reason: "is a directory" } })),
      opened(answer({ held: { version: V, format: "plain", bytes: 9, body: "opaque" } })),
      opened(answer({ empty: { version: V, format: "markdown" } })),
    ]).toEqual([
      { kind: "missing" },
      { kind: "unreadable", reason: "is a directory" },
      { kind: "opaque", version: V, bytes: 9 },
      {
        kind: "text",
        gathering: { version: V, format: "markdown", encoding: "utf8", bytes: 0, text: "", through: 0 },
      },
    ]);
  });
});

describe("gathering a version", () => {
  test("asks for the rest after the head, and stops once the windows meet the end", () => {
    const head = gathering(opened(headOf("# one\n\n", 12)));
    const first = nextSpan(head);
    const after = joined(head, { version: V, window: { span: { start: 7, end: 12 }, text: "two\n\n" } });
    expect([whole(head), first, whole(after), after.text, nextSpan(after)]).toEqual([
      false, { start: 7, end: 12 }, true, "# one\n\ntwo\n\n", null,
    ]);
  });

  // A window answered for another version, or one that arrives twice or
  // out of order, would splice somebody else's bytes into this text.
  test("leaves out a window of another version or one that does not follow on", () => {
    const head = gathering(opened(headOf("# one\n\n", 12)));
    expect([
      joined(head, { version: W, window: { span: { start: 7, end: 12 }, text: "two\n\n" } }),
      joined(head, { version: V, window: { span: { start: 9, end: 12 }, text: "o\n\n" } }),
    ]).toEqual([head, head]);
  });

  test("asks for nothing more of a version larger than the editable bound", () => {
    const head = gathering(opened(headOf("# one\n\n", EDITABLE_BYTES_MAX + 1)));
    expect([nextSpan(head), whole(head)]).toEqual([null, false]);
  });
});
