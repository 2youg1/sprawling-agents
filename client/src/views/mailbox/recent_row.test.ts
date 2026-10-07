// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// A recent session's row: the fork key is always drawn, and where the
// page no longer holds the session's last turn it is disabled and its
// hint says why instead of its name.

import { describe, expect, test } from "bun:test";

import { say } from "../../core/lang";
import { Seq, TimeMs } from "../../wire";
import type { SessionLine } from "../../wire";
import { rowOf } from "./recent_row";
import type { RowHands } from "./recent_row";

const LINE: SessionLine = { at: TimeMs.make(0), began: Seq.make(1), last: Seq.make(4), runs: 2, start: { dispatched: { by: null } } };
const HANDS: RowHands = { href: "#/talk/lab/east", follow: () => undefined, fork: () => undefined };

describe("a recent session's row", () => {
  test("links to the room as an entry, and says how the session began", () => {
    const look = rowOf({ room: "lab/east", line: LINE, forkable: true }, "en", "1 min ago", HANDS);
    expect(look.link).toEqual({ href: HANDS.href, onclick: HANDS.follow, "data-entry": "" });
    expect(look.began).toBe("dispatched · 2 runs");
  });

  test("forks from the last turn while the page holds it", () => {
    const look = rowOf({ room: "lab/east", line: LINE, forkable: true }, "en", "", HANDS);
    const name = say("en", "mailbox_fork_last");
    expect(look.fork).toEqual({
      name,
      why: undefined,
      wire: { type: "button", "aria-label": name, "aria-disabled": false, onclick: HANDS.fork },
    });
  });

  test("says why it cannot fork once the last turn is older than the page", () => {
    const look = rowOf({ room: "lab/east", line: LINE, forkable: false }, "en", "", HANDS);
    expect(look.fork.why).toBe("the last run of this session is older than what this page holds");
    expect(look.fork.wire["aria-disabled"]).toBe(true);
  });
});
