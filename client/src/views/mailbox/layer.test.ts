// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// Trace vectors for the properties `client/spec/Views/Workspace.lean`
// proves of the mailbox layer: every trace over the five inputs up to a
// length, from every state the shell can start in.

import { describe, expect, test } from "bun:test";

import { stepMail } from "./layer";
import type { Mail, MailInput } from "./layer";

const INPUTS: readonly MailInput[] = ["toggle", "escape", "outside", "enter", "follow"];
const STARTS: readonly Mail[] = [
  { shown: false, focus: "key" },
  { shown: false, focus: "elsewhere" },
];

function traces(length: number): MailInput[][] {
  if (length === 0) return [[]];
  return traces(length - 1).flatMap((trace) => INPUTS.map((input) => [...trace, input]));
}

function run(mail: Mail, inputs: readonly MailInput[]): Mail {
  return inputs.reduce(stepMail, mail);
}

const ALL = [0, 1, 2, 3, 4, 5].flatMap(traces);

describe("the mailbox layer", () => {
  test("no focus stays in a closed mailbox", () => {
    const stuck = STARTS.flatMap((start) =>
      ALL.filter((trace) => {
        const end = run(start, trace);
        return !end.shown && end.focus === "inside";
      }),
    );
    expect(stuck).toEqual([]);
  });

  test("escape closes and returns focus from inside", () => {
    const wrong = STARTS.flatMap((start) =>
      ALL.filter((trace) => run(start, trace).focus === "inside").filter((trace) => {
        const end = run(start, [...trace, "escape"]);
        return end.shown || end.focus !== "key";
      }),
    );
    expect(wrong).toEqual([]);
  });

  test("two presses come home", () => {
    expect(STARTS.map((start) => run(start, ["toggle", "toggle"]))).toEqual(STARTS);
  });

  test("a press outside closes", () => {
    const open = STARTS.flatMap((start) => ALL.filter((trace) => run(start, [...trace, "outside"]).shown));
    expect(open).toEqual([]);
  });

  test("the key opens a closed mailbox and leaves the focus", () => {
    expect(STARTS.map((start) => run(start, ["toggle"]))).toEqual(
      STARTS.map((start) => ({ ...start, shown: true })),
    );
  });
});
