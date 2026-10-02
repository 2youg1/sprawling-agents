// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { describe, expect, test } from "bun:test";

import type { Tail } from "../../core/live_output";
import { Locator, Seq, TimeMs } from "../../wire";
import type { Call } from "../../wire";
import { printedOf } from "./terminal";

const TAIL: Tail = { out: "Compiling city\n", err: "warning: unused\n", cut: 3 };

function exec(outcome: Call["outcome"], output: Call["output"], answered: number | null): Call {
  return {
    tool: "exec",
    subject: null,
    arguments: { head: JSON.stringify({ arm: { shell: { text: "cargo nextest -p city" } } }), cut: 0 },
    outcome,
    at: Seq.make(41),
    output,
    called: TimeMs.make(1_000),
    answered: answered === null ? null : TimeMs.make(answered),
    timing: "measured",
    effect: "egress",
    render: "terminal",
  };
}

describe("the inspector's terminal", () => {
  test("prints the live tail while the command runs", () => {
    expect(printedOf(exec("waiting", null, null), TAIL)).toEqual({
      line: "cargo nextest -p city",
      out: "Compiling city\n",
      err: "warning: unused\n",
      cut: 3,
      ending: { kind: "running" },
      took: null,
      finished: null,
      pinned: null,
      live: true,
    });
  });

  // The tail of a run that has moved on can still be in hand when the
  // result lands; the result is what the terminal prints from then on.
  test("drops the tail once the result lands, and prints the Ledger's", () => {
    const result = JSON.stringify({ stdout: "41 passed\n", stderr: "", exit_code: 0 });
    const pinned = `cas:b3-${"a".repeat(64)}`;
    expect(printedOf(exec("answered", { head: result, cut: 0, pinned }, 4_412), TAIL)).toEqual({
      line: "cargo nextest -p city",
      out: "41 passed\n",
      err: "",
      cut: 0,
      ending: { kind: "code", code: 0 },
      took: 3_412,
      finished: TimeMs.make(4_412),
      pinned: Locator.make(pinned),
      live: false,
    });
  });

  test("prints any other call as what it was called on and what it answered", () => {
    const search: Call = {
      ...exec("failed", { head: "no such directory", cut: 0 }, 1_020),
      tool: "search",
      subject: "crates/nowhere",
      render: "generic",
      effect: "read",
    };
    expect(printedOf(search, TAIL)).toEqual({
      line: "crates/nowhere",
      out: "",
      err: "no such directory",
      cut: 0,
      ending: null,
      took: 20,
      finished: TimeMs.make(1_020),
      pinned: null,
      live: false,
    });
  });
});
