// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { describe, expect, test } from "bun:test";

import {
  advance,
  backoffMs,
  connect,
  isStopped,
  newLink,
  type Link,
  type LinkEvent,
} from "./link";
import { B3Hash, WIRE_HASH, WIRE_V, type AxError } from "../wire";

// A link that has been greeted, which is where every frame below
// arrives.
function live(): Link {
  const [opening] = connect(newLink("en"));
  const [handshaking] = advance(opening, { kind: "opened", credential: null });
  const [settled] = advance(handshaking, {
    kind: "received",
    frame: {
      welcome: {
        wire_v: WIRE_V,
        schema: B3Hash.make(WIRE_HASH),
        resume_from: null,
        city: null,
      },
    },
  });
  return settled;
}

function refusalOf(code: AxError["code"]): AxError {
  return {
    code,
    action: "run a command",
    subject: "the city said no",
    recovery: "ask for something this building may do",
    nearby: [],
    retry: "no",
  };
}

describe("a frame this build cannot read", () => {
  // The defect this pins: an undecodable frame used to reach the
  // machine as a close, which is an outage, so the page reconnected
  // every 250 ms and said nothing. The two ends disagreeing about the
  // wire is permanent, and only the person can act on it.
  test("stops the link instead of retreating down the ladder", () => {
    const [stopped, action] = advance(live(), { kind: "undecodable" });
    expect(isStopped(stopped)).toBe(true);
    expect(action).toEqual({
      kind: "report",
      error: {
        code: "E_WIRE_MISMATCH",
        action: "read a frame from this city",
        subject: `this page speaks wire v${String(WIRE_V)} and the server sent a frame it cannot read`,
        recovery:
          "reload the page to fetch the client this server was built with",
        nearby: [],
        retry: "no",
      },
    });
  });

  // The ladder is cancelled as a state, not only as a timer: a wait
  // booked before the refusal fires into a machine that ignores it.
  test("a wait that was already booked reopens nothing", () => {
    const [stopped] = advance(live(), { kind: "undecodable" });
    expect(advance(stopped, { kind: "wait_elapsed" })).toEqual([
      stopped,
      { kind: "nothing" },
    ]);
    expect(advance(stopped, { kind: "closed" })).toEqual([
      stopped,
      { kind: "nothing" },
    ]);
  });

  test("the person's retry is what starts it again", () => {
    const [stopped] = advance(live(), { kind: "undecodable" });
    const [again, action] = advance(stopped, { kind: "retry" });
    expect(again.state).toEqual({ kind: "opening" });
    expect(action).toEqual({ kind: "open" });
  });
});

describe("a refusal the server sent", () => {
  test("about the wire ends the link", () => {
    const [stopped] = advance(live(), {
      kind: "received",
      frame: { refusal: refusalOf("E_WIRE_MISMATCH") },
    });
    expect(isStopped(stopped)).toBe(true);
  });

  // Everything else is one answer going wrong. The link carried it
  // faithfully, so the link stays up and the person reads the refusal.
  test("about one command leaves the link live", () => {
    const [carried, action] = advance(live(), {
      kind: "received",
      frame: { refusal: refusalOf("E_GATE_DENIED") },
    });
    expect(carried.state.kind).toBe("live");
    expect(action).toEqual({
      kind: "report",
      error: refusalOf("E_GATE_DENIED"),
    });
  });
});

describe("the backoff ladder", () => {
  // A real outage still retreats, and the rungs still grow: the count
  // is cleared by a frame arriving, not by a handshake completing, so
  // a socket that greets and dies before saying anything climbs.
  test("grows across sockets that greet and then die", () => {
    let link = live();
    const waits: number[] = [];
    for (let attempt = 0; attempt < 3; attempt += 1) {
      const [dropped, wait] = advance(link, { kind: "closed" });
      expect(wait).toEqual({ kind: "wait", ms: backoffMs(attempt) });
      waits.push(backoffMs(attempt));
      const [opening] = advance(dropped, { kind: "wait_elapsed" });
      const [handshaking] = advance(opening, { kind: "opened", credential: null });
      const [greeted] = advance(handshaking, {
        kind: "received",
        frame: {
          welcome: {
            wire_v: WIRE_V,
            schema: B3Hash.make(WIRE_HASH),
            resume_from: null,
            city: null,
          },
        },
      });
      link = greeted;
    }
    expect(waits).toEqual([250, 500, 1000]);
  });

  test("returns to its first rung once a frame arrives", () => {
    const [dropped] = advance(live(), { kind: "closed" });
    const [opening] = advance(dropped, { kind: "wait_elapsed" });
    const [handshaking] = advance(opening, { kind: "opened", credential: null });
    const [greeted] = advance(handshaking, {
      kind: "received",
      frame: {
        welcome: {
          wire_v: WIRE_V,
          schema: B3Hash.make(WIRE_HASH),
          resume_from: null,
          city: null,
        },
      },
    });
    const heard: LinkEvent = {
      kind: "received",
      frame: { refusal: refusalOf("E_GATE_DENIED") },
    };
    const [flowing] = advance(greeted, heard);
    expect(advance(flowing, { kind: "closed" })[1]).toEqual({
      kind: "wait",
      ms: 250,
    });
  });
});

// The person closed the city on purpose (`/quit`): every event the link
// can meet after that leaves it stopped, and only the person's retry
// starts it again, with the close forgotten (client/spec/Core/Link.lean).
describe("a city the person closed", () => {
  const EVENTS: readonly LinkEvent[] = [
    { kind: "opened", credential: null },
    { kind: "closed" },
    { kind: "wait_elapsed" },
    { kind: "undecodable" },
    { kind: "closing", mode: "drain" },
  ];

  for (const mode of ["drain", "interrupt"] as const) {
    test(`stays closed after a ${mode} close, whatever arrives, until the person retries`, () => {
      const [asked, quiet] = advance(live(), { kind: "closing", mode });
      expect(quiet).toEqual({ kind: "nothing" });
      const [ended, action] = advance(asked, { kind: "closed" });
      expect({ state: ended.state, action }).toEqual({ state: { kind: "closed" }, action: { kind: "nothing" } });
      for (const event of EVENTS) {
        const [after, done] = advance(ended, event);
        expect({ state: after.state, done, stopped: isStopped(after) }).toEqual({
          state: { kind: "closed" },
          done: { kind: "nothing" },
          stopped: true,
        });
      }
      const [again, opens] = advance(ended, { kind: "retry" });
      expect({ state: again.state, closing: again.closing, opens }).toEqual({
        state: { kind: "opening" },
        closing: null,
        opens: { kind: "open" },
      });
    });
  }

  test("a link the person did not close climbs the ladder as before", () => {
    expect(advance(live(), { kind: "closed" })[1]).toEqual({ kind: "wait", ms: 250 });
  });
});

describe("the hello", () => {
  test("shows the credential the line was opened under", () => {
    const [opening] = connect(newLink("en"));
    const [, said] = advance(opening, { kind: "opened", credential: "session-token" });
    expect(said).toEqual({
      kind: "send",
      frame: { hello: { wire_v: WIRE_V, schema: B3Hash.make(WIRE_HASH), token: "session-token" } },
    });
  });
});
