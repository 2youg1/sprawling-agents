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

import { Turn } from "../../wire";
import { traceOf } from "./trace";

const turn = (number: number, calls: readonly unknown[]): Turn =>
  Schema.decodeUnknownSync(Turn)({ number, calls, notes: [], opened: number * 10, t: number * 1000 });

const exec = turn(1, [
  {
    at: 11,
    tool: "exec",
    outcome: "answered",
    arguments: { cut: 0, head: JSON.stringify({ arm: { shell: { text: "cargo test" } } }, null, 2) },
    output: {
      cut: 0,
      head: JSON.stringify({ arm: "shell", stdout: "ok\n", stderr: "warn\n", exit_code: 101 }),
    },
  },
  {
    at: 12,
    tool: "exec",
    outcome: "waiting",
    arguments: { cut: 0, head: JSON.stringify({ arm: { program: { path: "git", args: ["status"] } } }) },
  },
]);

const edit = turn(2, [
  {
    at: 21,
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
  { at: 22, tool: "read", subject: "src/main.rs", outcome: "answered", arguments: { cut: 0, head: '{"path":"src/main.rs"}' } },
]);

describe("a run's turns read as a terminal record and a code column", () => {
  test("an exec call is its command, both streams and its exit code", () => {
    const { entries } = traceOf([exec]);
    expect(entries).toEqual([
      { kind: "command", at: 11, text: "cargo test", stdout: "ok\n", stderr: "warn\n", ending: { kind: "code", code: 101 }, cut: 0 },
      { kind: "command", at: 12, text: "git status", stdout: "", stderr: "", ending: { kind: "running" }, cut: 0 },
    ]);
  });

  test("an edit's hunk carries both line numbers, and following lands on the newest touch", () => {
    const trace = traceOf([exec, edit]);
    expect(trace.files).toEqual([
      {
        path: "src/lib.rs",
        how: "modified",
        at: 21,
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
    expect(trace.latest).toEqual({ kind: "entry", at: 22 });
    expect(traceOf([exec]).latest).toEqual({ kind: "entry", at: 12 });
    expect(traceOf([turn(2, edit.calls.slice(0, 1))]).latest).toEqual({ kind: "file", path: "src/lib.rs" });
  });
});
