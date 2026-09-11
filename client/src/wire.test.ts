// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What the generated wire is held to from the client's side: the frames
// the server accepts decode to the same values, and a frame it would
// refuse fails here with a message that names the field.

import { describe, expect, test } from "bun:test";
import { Either, ParseResult, Schema } from "effect";

import { ClientFrame, ServerFrame, WIRE_HASH, WIRE_V } from "./wire";

const run = "01923456-7890-7abc-8def-0123456789ab";
const idem = "idem1-0123456789abcdef0123456789abcdef";
const hash = "f".repeat(64);

const decodeClient = Schema.decodeUnknownEither(ClientFrame);
const decodeServer = Schema.decodeUnknownEither(ServerFrame);

// Both helpers hand back what an assertion can compare: the decoded value
// or the formatted refusal, so a failing test shows the tree the client
// would have shown.
const accepted = <A>(result: Either.Either<A, ParseResult.ParseError>): unknown =>
  Either.match(result, {
    onLeft: (error) => ParseResult.TreeFormatter.formatErrorSync(error),
    onRight: (value) => value,
  });

const refused = <A>(result: Either.Either<A, ParseResult.ParseError>): string =>
  Either.match(result, {
    onLeft: (error) => ParseResult.TreeFormatter.formatErrorSync(error),
    onRight: () => "decoded a frame the server would refuse",
  });

describe("the wire constants", () => {
  test("carry the version and the hash the server checks", () => {
    expect(WIRE_V).toBe(22);
    expect(WIRE_HASH).toMatch(/^[0-9a-f]{64}$/);
  });
});

describe("client frames", () => {
  test("a Hello, a Command and two Queries decode to what was sent", () => {
    const hello = { hello: { wire_v: WIRE_V, schema: hash, token: null } };
    const command = { command: { steer: { run, text: "keep going", idem } } };
    const paged = { query: { run_history: { run, before: null, limit: 20 } } };
    const unit = { query: "city_view" };
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
  test("an Answer with nested values, an Event and a Delta decode to what was sent", () => {
    const answer = {
      answer: {
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
          pursuits: [{ addr: "acme", goal: "ship it", state: "running", verdict: "working on 2.3" }],
          halted: [],
        },
      },
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
    const delta = { delta: { run, text: "tok" } };
    for (const frame of [answer, event, delta]) {
      expect(accepted(decodeServer(frame))).toEqual(frame);
    }
  });
});

describe("a malformed frame", () => {
  test("fails with a message that names the field and the mismatch", () => {
    const frame = { command: { steer: { run: 5, text: "x", idem } } };
    const message = refused(decodeClient(frame));
    // The tree is named at every level the wire names: the root, the
    // frame kind, then the field and what was found there.
    expect(message.startsWith("ClientFrame")).toBe(true);
    expect(message).toContain("└─ Command");
    expect(message).toContain('["steer"]');
    expect(message).toContain('["run"]');
    expect(message).toContain('Expected string & Brand<"RunId">, actual 5');
  });

  test("with a kind the wire does not know is refused by name", () => {
    const frame = { event: { v: 1, run, seq: 1, prev: hash, t: 0, who: "x", kind: "run_teleported", data: {} } };
    const message = refused(decodeServer(frame));
    expect(message).toContain('["kind"]');
    expect(message).toContain('"run_teleported"');
  });
});
