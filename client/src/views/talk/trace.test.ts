// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { describe, expect, test } from "bun:test";

import { Seq, TimeMs, type Call, type Turn } from "../../wire";
import { artifactsIn, deedOf, tally } from "./trace";

function call(tool: string, subject: string | null, head: string | null): Call {
  return {
    tool,
    subject,
    arguments: null,
    outcome: head === null ? "waiting" : "answered",
    at: Seq.make(1),
    output: head === null ? null : { head, cut: 0 },
  };
}

function turn(number: number, calls: readonly Call[]): Turn {
  return {
    calls,
    notes: [],
    number,
    opened: Seq.make(number),
    t: TimeMs.make(number),
  };
}

describe("what a run did", () => {
  test("the three classes the summary counts are the three L0 tools", () => {
    expect(deedOf("read")).toBe("explored");
    expect(deedOf("search")).toBe("explored");
    expect(deedOf("edit")).toBe("wrote");
    expect(deedOf("exec")).toBe("ran");
  });

  // A tool this build has no class for is still counted, so the
  // clauses of the summary add up to the calls that were made.
  test("a tool nobody classed is counted rather than dropped", () => {
    expect(deedOf("delegate")).toBe("other");
    expect(deedOf("apps_github_create_issue")).toBe("other");
    const counted = tally([
      call("read", "a.rs", "x"),
      call("read", "b.rs", "x"),
      call("edit", "a.rs", "x"),
      call("exec", "just check", "x"),
      call("delegate", null, "x"),
    ]);
    expect(counted).toEqual({ explored: 2, wrote: 1, ran: 1, other: 1 });
    expect(counted.explored + counted.wrote + counted.ran + counted.other).toBe(5);
  });
});

describe("what the panel shows", () => {
  test("a run that produced nothing gets no panel", () => {
    const held = artifactsIn([turn(1, [call("delegate", null, "sent")])]);
    expect(held.read).toBeNull();
    expect(held.wrote).toBeNull();
    expect(held.terminal).toBeNull();
  });

  // The three panes are filled independently, so a command run after a
  // file was read does not push the file out of the card.
  test("each pane holds the newest of its own kind", () => {
    const held = artifactsIn([
      turn(1, [call("read", "old.rs", "one")]),
      turn(2, [call("exec", "cargo build", "built"), call("read", "new.rs", "two")]),
      turn(3, [call("exec", "cargo test", "passed")]),
    ]);
    expect(held.read?.subject).toBe("new.rs");
    expect(held.terminal?.subject).toBe("cargo test");
  });

  // The pane a change lands in is not the pane a reading lands in.
  // Folded together, the later of the two won and the other was not
  // drawn at all - a run that rewrote a module and then read a header
  // showed the header.
  test("a file that was changed and a file that was read are two panes", () => {
    const held = artifactsIn([
      turn(1, [call("edit", "changed.rs", "one line")]),
      turn(2, [call("read", "looked.rs", "two")]),
    ]);
    expect(held.wrote?.subject).toBe("changed.rs");
    expect(held.read?.subject).toBe("looked.rs");
  });

  test("a call still waiting is not yet an artifact", () => {
    const held = artifactsIn([
      turn(1, [call("edit", "done.rs", "written")]),
      turn(2, [call("edit", "pending.rs", null)]),
    ]);
    expect(held.wrote?.subject).toBe("done.rs");
  });

  test("a call that answered with nothing is not an artifact either", () => {
    const held = artifactsIn([turn(1, [call("exec", "true", "")])]);
    expect(held.terminal).toBeNull();
  });
});
