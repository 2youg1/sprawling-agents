// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// A recent session's row: the fork key is always drawn, and where the
// page no longer holds the session's last turn it is disabled and its
// hint says why instead of its name. A branch from the row that waits on
// the city, or that the city refused, is said where the row says how the
// session began.

import { describe, expect, test } from "bun:test";

import { say } from "../../core/lang";
import { Address, Seq, TimeMs } from "../../wire";
import type { AxError, SessionLine } from "../../wire";
import { rowOf } from "./recent_row";
import type { RowHands } from "./recent_row";

const LINE: SessionLine = { at: TimeMs.make(0), began: Seq.make(1), last: Seq.make(4), runs: 2, start: { dispatched: { by: null } } };
const HANDS: RowHands = { href: "#/talk/lab/east", follow: () => undefined, fork: () => undefined, stop: null };
const ROOM = Address.make("lab/east");
const NONE = { kind: "none" } as const;
const STOP = (): void => undefined;

function refused(code: AxError["code"]): AxError {
  return { code, action: "start a new session", subject: "lab/east: a run is running there", recovery: "", nearby: [], retry: "no" };
}

describe("a recent session's row", () => {
  test("links to the room as an entry, and says how the session began", () => {
    const look = rowOf({ room: ROOM, line: LINE, forkable: true, branch: NONE }, "en", "1 min ago", HANDS);
    expect(look.link).toEqual({ href: HANDS.href, onclick: HANDS.follow, "data-entry": "" });
    expect(look.began).toBe("dispatched · 2 runs");
  });

  test("forks from the last turn while the page holds it", () => {
    const look = rowOf({ room: ROOM, line: LINE, forkable: true, branch: NONE }, "en", "", HANDS);
    const name = say("en", "mailbox_fork_last");
    expect(look.fork).toEqual({
      name,
      why: undefined,
      wire: { type: "button", "aria-label": name, "aria-disabled": false, onclick: HANDS.fork },
    });
  });

  test("says why it cannot fork once the last turn is older than the page", () => {
    const look = rowOf({ room: ROOM, line: LINE, forkable: false, branch: NONE }, "en", "", HANDS);
    expect(look.fork.why).toBe("the last run of this session is older than what this page holds");
    expect(look.fork.wire["aria-disabled"]).toBe(true);
  });

  test("says the branch is opening, and waits for it", () => {
    const look = rowOf({ room: ROOM, line: LINE, forkable: true, branch: { kind: "opening", of: "branch" } }, "en", "", HANDS);
    expect([look.began, look.fork.why, look.fork.wire["aria-disabled"], look.stop]).toEqual([
      say("en", "branch_opening"),
      say("en", "branch_opening"),
      true,
      null,
    ]);
  });

  test("waits while another row's branch is opening", () => {
    const look = rowOf({ room: ROOM, line: LINE, forkable: true, branch: { kind: "elsewhere" } }, "en", "", HANDS);
    expect([look.began, look.fork.why, look.fork.wire["aria-disabled"]]).toEqual([
      "dispatched · 2 runs",
      say("en", "branch_one_at_a_time"),
      true,
    ]);
  });

  test("says why a busy room refused the branch, with /stop for its run, and forks again", () => {
    const look = rowOf({ room: ROOM, line: LINE, forkable: true, branch: { kind: "refused", of: "branch", error: refused("E_BUSY") } }, "en", "", { ...HANDS, stop: STOP });
    expect([look.began, look.fork.wire["aria-disabled"], look.stop]).toEqual([
      say("en", "branch_busy"),
      false,
      { label: "/stop", name: "stop the run working in lab/east", onPress: STOP },
    ]);
  });

  test("offers no /stop once the room has no run working, nor for another refusal", () => {
    const busy = rowOf({ room: ROOM, line: LINE, forkable: true, branch: { kind: "refused", of: "branch", error: refused("E_BUSY") } }, "en", "", HANDS);
    const other = rowOf({ room: ROOM, line: LINE, forkable: true, branch: { kind: "refused", of: "branch", error: refused("E_INVALID_ARGS") } }, "en", "", { ...HANDS, stop: STOP });
    expect([busy.stop, other.stop]).toEqual([null, null]);
    expect(other.began.startsWith("not branched: ")).toBe(true);
  });
});
