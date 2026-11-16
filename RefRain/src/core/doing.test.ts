// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { describe, expect, test } from "bun:test";

import { afterWait, sendingInto, type Doing } from "./doing";
import { Address, TimeMs } from "../wire";
import type { ReplyEnd } from "../wire";

const AWAITING: Doing = {
  kind: "awaiting_reply",
  wait: { on: Address.make("lab/west"), until: TimeMs.make(60_000) },
};

describe("sending", () => {
  
  
  test("a run that is blocked queues the words instead of saying them", () => {
    const calling: Doing = { kind: "calling", tool: "exec", subject: "just check" };
    expect(sendingInto(calling)).toBe("queued");
    expect(sendingInto({ kind: "waiting" })).toBe("queued");
    
    expect(sendingInto(AWAITING)).toBe("queued");
  });

  test("a run between calls hears a steer at the next boundary", () => {
    expect(sendingInto({ kind: "thinking" })).toBe("steer");
  });

  
  test("a run whose phase this page was never told queues", () => {
    expect(sendingInto({ kind: "unknown" })).toBe("queued");
  });

  
  test("no live run and a frozen one both open new work", () => {
    expect(sendingInto(undefined)).toBe("dispatch");
    expect(sendingInto({ kind: "frozen", completion: "done" })).toBe("dispatch");
    expect(sendingInto({ kind: "frozen", completion: null })).toBe("dispatch");
  });
});


describe("the end of a reply wait", () => {
  test("a reply or a timeout hands the run back to its model, a run that left is told nothing yet", () => {
    const ends: readonly ReplyEnd[] = ["reply", "timeout", "left"];
    expect(ends.map((end) => afterWait(AWAITING, end))).toEqual([
      { kind: "thinking" },
      { kind: "thinking" },
      { kind: "unknown" },
    ]);
  });

  
  test("an ending leaves a run that is no longer waiting where it is", () => {
    const frozen: Doing = { kind: "frozen", completion: "cancelled" };
    expect(afterWait(frozen, "left")).toEqual(frozen);
    expect(afterWait({ kind: "calling", tool: "read", subject: null }, "reply")).toEqual({
      kind: "calling",
      tool: "read",
      subject: null,
    });
  });
});
