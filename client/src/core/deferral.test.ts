// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { describe, expect, test } from "bun:test";

import { IDLE_MS, RETURNED_MS, deliverable, momentOf, wakeAt } from "./deferral";
import type { Attention, Box } from "./deferral";

const AT = 100_000;

function seen(box: Box, returnedAt: number | null = null): Attention {
  return { box, visible: true, returnedAt };
}

const WRITING = seen({ kind: "holding" });

describe("momentOf", () => {
  test("a box emptied by a send is a moment at once, and stays one while it stays empty", () => {
    const sent = seen({ kind: "emptied", at: AT, by: "send" });
    expect([momentOf(sent, AT), momentOf(sent, AT + 60_000)]).toEqual(["sent", "sent"]);
  });

  test("a box emptied by hand becomes a moment only once it has stayed empty for IDLE_MS", () => {
    const cleared = seen({ kind: "emptied", at: AT, by: "hand" });
    expect([momentOf(cleared, AT + IDLE_MS - 1), momentOf(cleared, AT + IDLE_MS)]).toEqual([null, "idle"]);
  });

  test("words in the box are no moment, unless the person has just come back to the tab", () => {
    expect([
      momentOf(WRITING, AT),
      momentOf(seen({ kind: "holding" }, AT), AT + RETURNED_MS),
      momentOf(seen({ kind: "holding" }, AT), AT + RETURNED_MS + 1),
    ]).toEqual([null, "returned", null]);
  });

  test("a page with no box is always a moment, and a hidden tab never is", () => {
    const absent = seen({ kind: "absent" });
    expect([momentOf(absent, AT), momentOf({ ...absent, visible: false }, AT)]).toEqual(["idle", null]);
  });
});

describe("deliverable", () => {
  test("what needs the person is delivered whatever the page is doing; the ordinary waits for a moment", () => {
    const hidden: Attention = { ...WRITING, visible: false };
    expect([
      deliverable("needs_you", WRITING, AT),
      deliverable("needs_you", hidden, AT),
      deliverable("ordinary", WRITING, AT),
      deliverable("ordinary", seen({ kind: "emptied", at: AT, by: "send" }), AT),
    ]).toEqual([true, true, false, true]);
  });
});

describe("wakeAt", () => {
  test("names the instant a hand-cleared box will have been empty long enough, and nothing otherwise", () => {
    expect([
      wakeAt(seen({ kind: "emptied", at: AT, by: "hand" }), AT + 1_000),
      wakeAt(seen({ kind: "emptied", at: AT, by: "hand" }), AT + IDLE_MS),
      wakeAt(WRITING, AT),
      wakeAt({ box: { kind: "emptied", at: AT, by: "hand" }, visible: false, returnedAt: null }, AT),
    ]).toEqual([AT + IDLE_MS, null, null, null]);
  });
});
