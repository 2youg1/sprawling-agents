// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The monitor reads a run from the turns the wire already carries. The
// runtime writes an exec result and an edit result as compact JSON, and
// these cases hold the reading to that shape: a command, what it wrote
// to each stream and the code it ended with; an edit's hunk with its
// line numbers; and the newest thing the agent touched, which is where
// following lands.

import { describe, expect, test } from "bun:test";
import { Schema } from "effect";

import { Seq, Turn } from "../../wire";
import { traceOf } from "./trace";

const turn = (number: number, calls: readonly object[]): Turn =>
  Schema.decodeUnknownSync(Turn)({
    number,
    calls: calls.map((call) => ({ called: number * 1000, timing: "measured", ...call })),
    notes: [],
    opened: number * 10,
    t: number * 1000,
    timing: "measured",
  });

const at = (n: number): Seq => Seq.make(n);

const exec = turn(1, [
  {
    at: 11,
    tool: "exec",
    outcome: "answered",
    called: 1000,
    answered: 1250,
    exit_code: 101,
    arguments: { cut: 0, head: JSON.stringify({ arm: { shell: { text: "cargo test" } } }, null, 2) },
    output: {
      cut: 0,
      head: JSON.stringify({ arm: "shell", stdout: "ok\n", stderr: "warn\n", exit_code: 101 }),
    },
  },
  {
    at: 12,
    called: 1020,
    tool: "exec",
    outcome: "waiting",
    arguments: { cut: 0, head: JSON.stringify({ arm: { program: { path: "git", args: ["status"] } } }) },
  },
]);

const edit = turn(2, [
  {
    at: 21,
    called: 2010,
    tool: "edit",
    subject: "src/lib.rs",
    outcome: "answered",
    arguments: { cut: 0, head: "{}" },
    output: {
      cut: 0,
      head: JSON.stringify({
        path: "src/lib.rs",
        base_version: "abc",
        new_version: "def",
        diff: "--- a/src/lib.rs\n+++ b/src/lib.rs\n@@ -4,1 +4,2 @@\n-let a = 1;\n+let a = 2;\n+let b = 3;\n",
      }),
    },
  },
  { at: 22, called: 2020, tool: "read", subject: "src/main.rs", outcome: "answered", arguments: { cut: 0, head: '{"path":"src/main.rs"}' } },
]);

describe("a run's turns read as a terminal record and a code column", () => {
  test("an exec call is its command, both streams, its exit code and how long it took", () => {
    const { entries } = traceOf([exec]);
    expect(entries).toEqual([
      { kind: "command", at: at(11), text: "cargo test", stdout: "ok\n", stderr: "warn\n", ending: { kind: "code", code: 101 }, cut: 0, took: 250 },
      { kind: "command", at: at(12), text: "git status", stdout: "", stderr: "", ending: { kind: "running" }, cut: 0, took: null },
    ]);
  });

  // The line the wire carries can be cut short of valid JSON; the code
  // the command ended with is the call's own and survives the cut.
  test("a command whose result line was cut still ends with the code its call carries", () => {
    const cut = turn(3, [
      {
        at: 31,
        tool: "exec",
        outcome: "answered",
        called: 3000,
        answered: 3100,
        exit_code: 2,
        arguments: { cut: 0, head: JSON.stringify({ arm: { shell: { text: "just check" } } }) },
        output: { cut: 40, head: '{"stdout":"error[E0308]' },
      },
    ]);
    expect(traceOf([cut]).entries.map((entry) => (entry.kind === "command" ? entry.ending : null))).toEqual([
      { kind: "code", code: 2 },
    ]);
  });

  test("an edit's hunk carries both line numbers, and following lands on the newest touch", () => {
    const trace = traceOf([exec, edit]);
    expect(trace.files).toEqual([
      {
        path: "src/lib.rs",
        how: "modified",
        at: at(21),
        hunks: [
          {
            lines: [
              { sign: "removed", old: 4, new: null, text: "let a = 1;" },
              { sign: "added", old: null, new: 4, text: "let a = 2;" },
              { sign: "added", old: null, new: 5, text: "let b = 3;" },
            ],
          },
        ],
      },
    ]);
    expect(trace.latest).toEqual({ kind: "entry", at: at(22) });
    expect(traceOf([exec]).latest).toEqual({ kind: "entry", at: at(12) });
    expect(traceOf([turn(2, edit.calls.slice(0, 1))]).latest).toEqual({ kind: "file", path: "src/lib.rs" });
  });

  test("a no-newline marker is neither a line nor a step of either counter", () => {
    const diff = "@@ -1,1 +1,2 @@\n-end\n\\ No newline at end of file\n+end\n+tail\n\\ No newline at end of file\n";
    const output = { cut: 0, head: JSON.stringify({ path: "a.txt", base_version: "abc", diff }) };
    const marked = turn(3, [{ at: 31, tool: "edit", outcome: "answered", arguments: { cut: 0, head: "{}" }, output }]);
    expect(traceOf([marked]).files.flatMap((file) => file.hunks)).toEqual([
      {
        lines: [
          { sign: "removed", old: 1, new: null, text: "end" },
          { sign: "added", old: null, new: 1, text: "end" },
          { sign: "added", old: null, new: 2, text: "tail" },
        ],
      },
    ]);
  });
});
