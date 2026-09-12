// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The link, as a value: where the connection is, what just happened to
// it, and what the browser half should do next. Every judgement is here
// and none is in `socket.ts`, so the ladder and the handshake are tested
// without a socket. Ported from `crates/web/src/socket/link.rs`.

import type {
  Answer,
  AxError,
  ClientFrame,
  Delta,
  EventRecord,
  LogLine,
  ServerFrame,
  Welcome,
} from "../wire";
import { B3Hash, WIRE_HASH, WIRE_V } from "../wire";

const SCHEMA: B3Hash = B3Hash.make(WIRE_HASH);

// The backoff ladder in milliseconds. Ends flat rather than growing
// without bound: a person who left the laptop closed should find the
// page live within a minute of opening it.
const LADDER_MS: readonly number[] = [250, 500, 1000, 2000, 5000, 10000];

export type LinkState =
  | { readonly kind: "idle" }
  | { readonly kind: "opening" }
  | { readonly kind: "handshaking" }
  | { readonly kind: "live"; readonly city: string | null }
  | { readonly kind: "backoff"; readonly attempt: number }
  | { readonly kind: "refused"; readonly error: AxError };

export type LinkEvent =
  | { readonly kind: "opened" }
  | { readonly kind: "received"; readonly frame: ServerFrame }
  | { readonly kind: "closed" }
  | { readonly kind: "wait_elapsed" }
  | { readonly kind: "retry" };

// Exhaustive, so the browser half cannot invent an action the machine
// never authorised.
export type LinkAction =
  | { readonly kind: "nothing" }
  | { readonly kind: "open" }
  | { readonly kind: "send"; readonly frame: ClientFrame }
  | { readonly kind: "welcomed"; readonly welcome: Welcome }
  | { readonly kind: "deliver"; readonly event: EventRecord }
  | { readonly kind: "answered"; readonly answer: Answer }
  | { readonly kind: "saying"; readonly delta: Delta }
  // One line of the process log. Not history: it has no sequence of
  // its own, it is never written down, and a page that missed one has
  // lost nothing.
  | { readonly kind: "logged"; readonly line: LogLine }
  | { readonly kind: "wait"; readonly ms: number }
  | { readonly kind: "report"; readonly error: AxError }
  | { readonly kind: "close" };

export interface Link {
  readonly state: LinkState;
  readonly token: string | null;
  // Failures since frames last flowed. On the link rather than in the
  // backoff state: every retry passes through `opening` on its way back
  // to `backoff`, and a counter living in the phase never climbs.
  readonly failures: number;
}

export function newLink(token: string | null): Link {
  return { state: { kind: "idle" }, token, failures: 0 };
}

export function isLive(link: Link): boolean {
  return link.state.kind === "live";
}

// The wait for one attempt number, clamped to the end of the ladder.
export function backoffMs(attempt: number): number {
  const last = LADDER_MS.length - 1;
  return LADDER_MS[Math.min(Math.max(attempt, 0), last)] ?? 10000;
}

function refusal(action: string, subject: string, recovery: string): AxError {
  return {
    code: "E_WIRE_MISMATCH",
    action,
    subject,
    recovery,
    nearby: [],
    retriable: false,
  };
}

function mismatch(welcome: Welcome): AxError {
  return refusal(
    "join this city's control surface",
    `this page speaks wire v${String(WIRE_V)} and the server speaks v${String(welcome.wire_v)}`,
    "reload the page to fetch the client this server was built with",
  );
}

const OUT_OF_ORDER = refusal(
  "join this city's control surface",
  "the server streamed frames before completing the handshake",
  "reload the page; if it repeats, the address is not a sprawling server",
);

// Starts, or restarts after a refusal was cleared by the person.
export function connect(link: Link): [Link, LinkAction] {
  return [{ ...link, state: { kind: "opening" } }, { kind: "open" }];
}

function refuse(link: Link, error: AxError): [Link, LinkAction] {
  return [
    { ...link, state: { kind: "refused", error } },
    { kind: "report", error },
  ];
}

function retreat(link: Link): [Link, LinkAction] {
  const attempt = link.failures;
  return [
    { ...link, failures: attempt + 1, state: { kind: "backoff", attempt } },
    { kind: "wait", ms: backoffMs(attempt) },
  ];
}

function welcomed(link: Link, welcome: Welcome): [Link, LinkAction] {
  if (welcome.wire_v !== WIRE_V || welcome.schema !== WIRE_HASH) {
    return refuse(link, mismatch(welcome));
  }
  // Frames flow again: the next outage starts at the bottom of the
  // ladder rather than inheriting an old grudge.
  return [
    { ...link, failures: 0, state: { kind: "live", city: welcome.city ?? null } },
    { kind: "welcomed", welcome },
  ];
}

function received(link: Link, frame: ServerFrame): [Link, LinkAction] {
  if (link.state.kind === "handshaking") {
    if ("welcome" in frame) {
      return welcomed(link, frame.welcome);
    }
    if ("refusal" in frame) {
      return refuse(link, frame.refusal);
    }
    // A server that streams or answers before welcoming is not speaking
    // this protocol; treat it as the mismatch it is.
    return refuse(link, OUT_OF_ORDER);
  }
  if (link.state.kind !== "live") {
    return [link, { kind: "nothing" }];
  }
  if ("event" in frame) {
    return [link, { kind: "deliver", event: frame.event }];
  }
  if ("answer" in frame) {
    return [link, { kind: "answered", answer: frame.answer }];
  }
  if ("delta" in frame) {
    return [link, { kind: "saying", delta: frame.delta }];
  }
  if ("log" in frame) {
    return [link, { kind: "logged", line: frame.log }];
  }
  if ("refusal" in frame) {
    return [link, { kind: "report", error: frame.refusal }];
  }
  return [link, { kind: "nothing" }];
}

// Advances the machine by one event.
export function advance(link: Link, event: LinkEvent): [Link, LinkAction] {
  const { state } = link;
  if (state.kind === "refused") {
    return event.kind === "retry"
      ? connect({ ...link, failures: 0 })
      : [link, { kind: "nothing" }];
  }
  switch (event.kind) {
    case "opened":
      if (state.kind !== "opening") {
        return [link, { kind: "nothing" }];
      }
      return [
        { ...link, state: { kind: "handshaking" } },
        {
          kind: "send",
          frame: {
            hello: { wire_v: WIRE_V, schema: SCHEMA, token: link.token },
          },
        },
      ];
    case "received":
      return received(link, event.frame);
    case "closed":
      return retreat(link);
    case "wait_elapsed":
      return state.kind === "backoff"
        ? connect(link)
        : [link, { kind: "nothing" }];
    case "retry":
      return connect({ ...link, failures: 0 });
  }
}
