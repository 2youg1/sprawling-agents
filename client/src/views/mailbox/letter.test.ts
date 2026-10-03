// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// Trace vectors for the letter properties `client/spec/Views/Workspace.lean`
// proves (client D73): every trace of mailbox inputs between opening a
// letter and closing it, up to a length, from every state the shell
// can start in.

import { describe, expect, test } from "bun:test";

import { stepLetter } from "./layer";
import type { LetterInput, LetterSide, Mail, MailInput } from "./layer";

const MAIL: readonly MailInput[] = ["toggle", "escape", "outside", "enter", "follow"];
const INPUTS: readonly LetterInput<string>[] = [
  ...MAIL.map((input): LetterInput<string> => ({ kind: "mail", input })),
  { kind: "open", row: "a" },
  { kind: "open", row: "b" },
  { kind: "close" },
];
const MAILS: readonly Mail[] = [
  { shown: false, focus: "key" },
  { shown: false, focus: "elsewhere" },
  { shown: true, focus: "inside" },
];
const STARTS: readonly LetterSide<string>[] = MAILS.map((mail) => ({ mail, opener: null, row: null }));

function traces<T>(alphabet: readonly T[], length: number): T[][] {
  if (length === 0) return [[]];
  return traces(alphabet, length - 1).flatMap((trace) => alphabet.map((input) => [...trace, input]));
}

function run(side: LetterSide<string>, inputs: readonly LetterInput<string>[]): LetterSide<string> {
  return inputs.reduce(stepLetter<string>, side);
}

describe("a letter on the right side", () => {
  test("closing a letter returns to its row", () => {
    const between = [0, 1, 2, 3].flatMap((length) => traces(MAIL, length));
    const wrong = STARTS.flatMap((start) =>
      between.filter((trace) => {
        const inputs: LetterInput<string>[] = [{ kind: "open", row: "a" }, ...trace.map((input): LetterInput<string> => ({ kind: "mail", input })), { kind: "close" }];
        const end = run(start, inputs);
        return !(end.mail.shown && end.mail.focus === "inside" && end.row === "a" && end.opener === null);
      }),
    );
    expect(wrong).toEqual([]);
  });

  test("letters hold the mail focus", () => {
    const all = [0, 1, 2, 3, 4].flatMap((length) => traces(INPUTS, length));
    const stuck = STARTS.flatMap((start) =>
      all.filter((trace) => {
        const end = run(start, trace).mail;
        return !end.shown && end.focus === "inside";
      }),
    );
    expect(stuck).toEqual([]);
  });
});
