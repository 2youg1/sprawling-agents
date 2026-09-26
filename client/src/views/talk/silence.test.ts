// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { describe, expect, test } from "bun:test";

import { Seq, TimeMs, type Note, type Turn } from "../../wire";
import { silentRun, silentTurn } from "./silence";

// A turn as the rounds carry it: `stopped` stays null until the model's
// answer is back, so an open turn is one still being waited on.
function turn(stopped: string | null, notes: readonly Note[] = []): Turn {
  return { calls: [], notes, number: 1, opened: Seq.make(1), t: TimeMs.make(1), said: null, stopped };
}

const refusedByProvider: Note = {
  refused: {
    at: Seq.make(2),
    error: {
      action: "call the model",
      code: "E_PROVIDER",
      nearby: [],
      recovery: "",
      retriable: true,
      subject: "main",
    },
  },
};

describe("when the thread says the model said nothing", () => {
  test("not while the first token is awaited or the reply is streaming", () => {
    expect(silentTurn(turn(null), "live")).toBe(false);
    expect(silentRun([turn(null)], "live")).toBe(false);
  });

  test("not when the provider refused the call, whose own box gives the reason", () => {
    const failed = [turn(null, [refusedByProvider])];
    expect(silentTurn(failed[0] ?? turn(null), "frozen")).toBe(false);
    expect(silentRun(failed, "frozen")).toBe(false);
  });

  test("still for a run that finished without a word", () => {
    expect(silentRun([turn("max_tokens")], "frozen")).toBe(true);
    expect(silentTurn(turn("end_turn"), "live")).toBe(true);
  });
});
