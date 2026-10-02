// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { describe, expect, test } from "bun:test";

import { Address, GitOid, RunId, Seq, TimeMs } from "../../wire";
import type { Call, RoundsAnswer } from "../../wire";
import { bracketOf } from "../checkpoints";
import { followingIn, readingOf } from "./reading";

const RUN = RunId.make("0199c0de-0000-4000-8000-000000000001");

function call(at: number, rest: Partial<Call> & Pick<Call, "tool">): Call {
  return {
    subject: null,
    arguments: null,
    outcome: "answered",
    at: Seq.make(at),
    output: { head: "ok", cut: 0 },
    called: TimeMs.make(at),
    answered: TimeMs.make(at + 1),
    timing: "measured",
    ...rest,
  };
}

const READ = call(1, {
  tool: "read",
  effect: "read",
  render: "generic",
  arguments: { head: JSON.stringify({ path: "shop/main.rs", offset: 9 }), cut: 0 },
});
const SEARCH = call(2, {
  tool: "search",
  effect: "read",
  render: "generic",
  arguments: { head: JSON.stringify({ pattern: "fn main", path: "shop" }), cut: 0 },
});
const EDIT = call(3, { tool: "edit", effect: { write: { domain: Address.make("shop") } }, render: { diff: { locations: [] } } });
const EXEC = call(4, { tool: "exec", effect: "egress", render: "terminal", outcome: "waiting", answered: null });
const SHOT = call(5, {
  tool: "browser",
  effect: "egress",
  render: "generic",
  output: { head: JSON.stringify({ image: `cas:b3-${"0".repeat(64)}`, width: 1280, height: 720, media_type: "image/png" }), cut: 0 },
});
const UNPLACED = call(6, { tool: "mcp/github" });

describe("how the inspector reads a call", () => {
  // The registration names terminals and diffs; the two other readings
  // are told apart by what the call recorded, never by its tool's name.
  test("by the registration first, then by the shape the call recorded", () => {
    expect([READ, SEARCH, EDIT, EXEC, SHOT, UNPLACED].map(readingOf)).toEqual(["file", "printed", "diff", "terminal", "shot", "printed"]);
  });

  test("follows the newest answered file call above the newest command, running or not", () => {
    const rounds: RoundsAnswer = {
      run: RUN,
      turns: [{ calls: [READ, EDIT, SEARCH, EXEC, UNPLACED], notes: [], number: 1, opened: Seq.make(0), t: TimeMs.make(0), timing: "measured" }],
    };
    expect(followingIn(rounds)).toEqual([
      { kind: "call", run: RUN, at: EDIT.at },
      { kind: "call", run: RUN, at: EXEC.at },
    ]);
  });
});

describe("the checkpoints around a call", () => {
  const OPENED = GitOid.make("0".repeat(40));
  const FIRST = GitOid.make("1".repeat(40));
  const SECOND = GitOid.make("2".repeat(40));
  const rounds: RoundsAnswer = {
    run: RUN,
    opened_at: OPENED,
    turns: [
      { calls: [], notes: [{ checkpointed: { at: Seq.make(10), oid: FIRST } }], number: 1, opened: Seq.make(0), t: TimeMs.make(0), timing: "measured" },
      { calls: [], notes: [{ checkpointed: { at: Seq.make(20), oid: SECOND } }], number: 2, opened: Seq.make(11), t: TimeMs.make(0), timing: "measured" },
    ],
  };

  test("are the newest one before it, or the opening tree, and the first one after it", () => {
    expect([bracketOf(rounds, Seq.make(5)), bracketOf(rounds, Seq.make(15)), bracketOf(rounds, Seq.make(25))]).toEqual([
      { base: OPENED, head: FIRST },
      { base: FIRST, head: SECOND },
      { base: SECOND, head: null },
    ]);
  });
});
