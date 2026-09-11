// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { describe, expect, test } from "bun:test";

import { sendingInto, type Doing } from "./belief";

describe("sending", () => {
  // The defect this pins is one of wording, and it is the one a
  // streaming page invites: text arriving letter by letter reads as a
  // conversation, and a person types into it expecting to be heard. A
  // steer is consumed at a phase boundary, so while a tool call is out
  // the words wait - and the composer has to say so.
  test("a run that is blocked queues the words instead of saying them", () => {
    const calling: Doing = { kind: "calling", tool: "exec", subject: "just check" };
    expect(sendingInto(calling)).toBe("queued");
    expect(sendingInto({ kind: "waiting" })).toBe("queued");
  });

  test("a run between calls hears a steer at the next boundary", () => {
    expect(sendingInto({ kind: "thinking" })).toBe("steer");
  });

  // Nothing in flight and a run that has ended are the same fact for a
  // person writing: the next message opens work rather than interrupting
  // it. `undefined` is "this room has no live run", not "unknown".
  test("no live run and a frozen one both open new work", () => {
    expect(sendingInto(undefined)).toBe("dispatch");
    expect(sendingInto({ kind: "frozen", completion: "done" })).toBe("dispatch");
    expect(sendingInto({ kind: "frozen", completion: null })).toBe("dispatch");
  });
});
