// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { describe, expect, test } from "bun:test";

import { fill, say } from "../../core/lang";
import { readingOf } from "../inspect/reading";
import { Seq, TimeMs, type Call } from "../../wire";
import { kindOf, lineOf, outcomeOf } from "./call_kind";

function call(render: Call["render"], args: object, said: object | null, cut = 0): Call {
  return {
    tool: "anything",
    // The subject the record carries today: the first string in key order.
    subject: "send",
    arguments: { head: JSON.stringify(args, null, 2), cut },
    output: said === null ? null : { head: JSON.stringify(said), cut: 0 },
    outcome: said === null ? "waiting" : "ok",
    at: Seq.make(9),
    called: TimeMs.make(1000),
    answered: said === null ? null : TimeMs.make(1200),
    timing: "measured",
    effect: render === "delegate" ? "spawn" : { write: { domain: "lab/potter" } },
    render,
  };
}

const sent = call("signal", { action: "send", to: "lab/kiln", text: "glaze is dry\nfire at noon" }, { id: "r-s1", to: "lab/kiln", kind: "mention", delivered: true });
const waited = call("signal", { action: "send", to: "lab/kiln", text: "ready?", wait: true }, {
  id: "r-s2",
  to: "lab/kiln",
  kind: "mention",
  delivered: true,
  waiting: "a reply from lab/kiln",
});
const pulled = call("signal", { action: "pull" }, { signals: [] });
const handed = call("delegate", { room: "lab/helper", task: "measure the kiln", goal: "a number, then stop" }, { room: "lab/helper", starts: "when this turn settles" });

function words(of: Call): string {
  const kind = kindOf(of);
  const line = lineOf(of);
  const said = outcomeOf(of);
  return [
    kind.kind === "registered" ? say("en", kind.word) : kind.tool,
    line.kind === "subject" ? line.subject : fill(say("en", line.word), line.fills),
    said === null ? "" : say("en", said),
  ].join(" | ");
}

describe("a send and a delegation read as what they did (client D85)", () => {
  test("a send names who it went to, the start of what it said, and that it went out", () => {
    expect(words(sent)).toBe("send | to lab/kiln: glaze is dry | sent");
  });

  test("a synchronous send says it waits for the reply", () => {
    expect(words(waited)).toBe("send | to lab/kiln: ready? | awaiting reply");
  });

  test("a pull is a pull, not a write", () => {
    expect(words(pulled)).toBe("pull | send | ");
  });

  test("a delegation names the room and the task, and when it starts", () => {
    expect(words(handed)).toBe("delegate | to lab/helper: measure the kiln | starts after this turn");
  });

  test("arguments cut by the window fall back to the subject rather than a guess", () => {
    expect(lineOf(call("signal", { action: "send", to: "lab/kiln", text: "x" }, null, 3))).toEqual({ kind: "subject", subject: "send" });
  });

  test("a send in flight has no outcome yet", () => {
    expect(outcomeOf(call("signal", { action: "send", to: "lab/kiln", text: "x" }, null))).toBeNull();
  });

  test("neither is drawn on the right side as a diff", () => {
    expect([readingOf(sent), readingOf(handed)]).toEqual(["printed", "printed"]);
  });
});
