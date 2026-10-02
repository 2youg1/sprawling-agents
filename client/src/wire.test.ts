// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What the generated wire is held to from the client's side: the frames
// the server accepts decode to the same values, and a frame it would
// refuse fails here with a message that names the field.

import { describe, expect, test } from "bun:test";
import { Result, Schema } from "effect";

import { ClientFrame, ServerFrame, WIRE_HASH, WIRE_V } from "./wire";

const run = "01923456-7890-7abc-8def-0123456789ab";
const idem = "idem1-0123456789abcdef0123456789abcdef";
const hash = "f".repeat(64);

const decodeClient = Schema.decodeUnknownResult(ClientFrame);
const decodeServer = Schema.decodeUnknownResult(ServerFrame);

// Both helpers hand back what an assertion can compare: the decoded value
// or the formatted refusal, so a failing test shows the tree the client
// would have shown.
const accepted = <A>(result: Result.Result<A, Schema.SchemaError>): unknown =>
  Result.match(result, {
    onFailure: (error) => error.message,
    onSuccess: (value) => value,
  });

const refused = <A>(result: Result.Result<A, Schema.SchemaError>): string =>
  Result.match(result, {
    onFailure: (error) => error.message,
    onSuccess: () => "decoded a frame the server would refuse",
  });

describe("the wire constants", () => {
  // The version's own value belongs to `wire::WIRE_V`, and
  // `xtask wire-ts` is what holds this file equal to it; repeating the
  // number here would be a second home that only ever disagrees. The
  // hash is text rather than a number, so its shape is still worth one
  // assertion: a truncated one would be refused by every server with
  // no clue as to why.
  test("carry a hash of the shape the handshake sends", () => {
    expect(WIRE_HASH).toMatch(/^[0-9a-f]{64}$/);
  });
});

describe("client frames", () => {
  test("a Hello, a Command and two Asks decode to what was sent", () => {
    const hello = { hello: { wire_v: WIRE_V, schema: hash, token: null } };
    const command = { command: { steer: { run, text: "keep going", idem } } };
    const paged = { ask: { ask_id: 1, query: { run_history: { run, before: null, limit: 20 } } } };
    const unit = { ask: { ask_id: 2, query: "city_view" } };
    for (const frame of [hello, command, paged, unit]) {
      expect(accepted(decodeClient(frame))).toEqual(frame);
    }
  });

  test("a credential over the wire is refused, as it is on the server", () => {
    const frame = { command: { put_secret: { realm: "anthropic", name: "api", value: "sk-nope" } } };
    expect(refused(decodeClient(frame))).toContain("put_secret");
  });
});

describe("server frames", () => {
  test("an Answered with nested values, an Event and a Delta decode to what was sent", () => {
    const answer = {
      answered: { ask_id: 3, as_of: 7, outcome: { answer: {
        city: {
          runs: [{ run, who: "planner@acme.1", frozen: false, last_seq: 7, last_kind: "run_started" }],
          active: 1,
          frozen: 0,
          buildings: [
            {
              addr: "acme",
              progress: { planned: { done: 1, blocked: 0, total: 3, done_ppb: 333333333, blocked_ppb: 0 } },
              problems: [],
              blocked: [{ source: "2.3", line: "2.3 waits for 2.1", waiting: 2 }],
              ready: 1,
            },
          ],
          pursuits: [{ addr: "acme", goal: "ship it", state: "running", verdict: { kind: "work", next: "2.3" } }],
          halted: [],
        },
      } } },
    };
    const event = {
      event: {
        v: 1,
        run,
        seq: 7,
        prev: hash,
        t: 1700000000000,
        who: "planner@acme.1",
        kind: "run_started",
        data: { task: "ship it", nested: { count: 2 } },
      },
    };
    const delta = { delta: { run, increment: { said: "tok" } } };
    for (const frame of [answer, event, delta]) {
      expect(accepted(decodeServer(frame))).toEqual(frame);
    }
  });
});

describe("a malformed frame", () => {
  test("fails with a message that names the field and the mismatch", () => {
    const frame = { command: { steer: { run: 5, text: "x", idem } } };
    const message = refused(decodeClient(frame));
    // Effect 4 states each failed branch as what it expected and the
    // path it expected it at, from the frame's own root down to the field.
    expect(message).toContain('Expected string\n  at ["command"]["steer"]["run"]');
  });

  test("with a kind the wire does not know is refused at the field, with the kinds it knows", () => {
    const frame = { event: { v: 1, run, seq: 1, prev: hash, t: 0, who: "x", kind: "run_teleported", data: {} } };
    const message = refused(decodeServer(frame));
    expect(message).toContain('at ["event"]["kind"]');
    expect(message).toContain('Expected "city_initialized" | ');
  });
});
