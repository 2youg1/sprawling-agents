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
  Seq,
  ServerFrame,
  Welcome,
} from "../wire";
import type { Key, Lang } from "./lang";
import { fill, say } from "./lang";
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
  // A frame arrived that this build cannot read. Its own event, because
  // it is not an outage: reconnecting meets the same frame again.
  | { readonly kind: "undecodable" }
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
  // A range of ledger records the event stream skipped. Its own action
  // rather than a report: the page can do something about it, and what
  // it does is ask the Ledger for the range.
  | { readonly kind: "lagged"; readonly from: Seq; readonly to: Seq }
  | { readonly kind: "wait"; readonly ms: number }
  | { readonly kind: "report"; readonly error: AxError }
  | { readonly kind: "close" };

export interface Link {
  readonly state: LinkState;
  readonly token: string | null;
  // Which language this page reads. On the link because the refusals
  // it mints are sentences a person meets, and `lang.json` is where
  // every one of those lives; a reducer that built them in English
  // would be a second place the client's words come from.
  readonly lang: Lang;
  // Failures since frames last flowed. On the link rather than in the
  // backoff state: every retry passes through `opening` on its way back
  // to `backoff`, and a counter living in the phase never climbs.
  readonly failures: number;
}

export function newLink(token: string | null, lang: Lang): Link {
  return { state: { kind: "idle" }, token, lang, failures: 0 };
}

export function isLive(link: Link): boolean {
  return link.state.kind === "live";
}

// Whether this link has stopped. A refused link never reconnects by
// itself: the socket half reads this to cancel a scheduled attempt and
// to drop the socket, and only the person's retry starts it again.
export function isRefused(link: Link): boolean {
  return link.state.kind === "refused";
}

// The wait for one attempt number, clamped to the end of the ladder.
export function backoffMs(attempt: number): number {
  const last = LADDER_MS.length - 1;
  return LADDER_MS[Math.min(Math.max(attempt, 0), last)] ?? 10000;
}

function refusal(
  lang: Lang,
  action: Key,
  subject: string,
  recovery: Key,
): AxError {
  return {
    code: "E_WIRE_MISMATCH",
    action: say(lang, action),
    subject,
    recovery: say(lang, recovery),
    nearby: [],
    retriable: false,
  };
}

function mismatch(lang: Lang, welcome: Welcome): AxError {
  return refusal(
    lang,
    "wire_join_action",
    fill(say(lang, "wire_version_mismatch"), {
      ours: String(WIRE_V),
      theirs: String(welcome.wire_v),
    }),
    "wire_reload_for_client",
  );
}

function outOfOrder(lang: Lang): AxError {
  return refusal(
    lang,
    "wire_join_action",
    say(lang, "wire_out_of_order"),
    "wire_reload_or_wrong_host",
  );
}

// A frame this build cannot read. Reconnecting cannot help: the server
// sends the same frame to the page it comes back as, so the ladder
// would run for ever behind a blank screen while the one thing that
// fixes it — fetching the client this server was built with — is a
// reload away.
function unreadable(lang: Lang): AxError {
  return refusal(
    lang,
    "wire_read_frame_action",
    fill(say(lang, "wire_frame_undecodable"), { ours: String(WIRE_V) }),
    "wire_reload_for_client",
  );
}

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
    return refuse(link, mismatch(link.lang, welcome));
  }
  return [
    { ...link, state: { kind: "live", city: welcome.city ?? null } },
    { kind: "welcomed", welcome },
  ];
}

// A refusal the server sent about the wire itself is the one refusal
// that ends the link: every later frame is at risk of the same
// disagreement, and a page that keeps asking only repeats it.
function refused(link: Link, error: AxError): [Link, LinkAction] {
  return error.code === "E_WIRE_MISMATCH"
    ? refuse(link, error)
    : [link, { kind: "report", error }];
}

function received(source: Link, frame: ServerFrame): [Link, LinkAction] {
  // Frames are flowing: the next outage starts at the bottom of the
  // ladder rather than inheriting an old grudge. Counted from data
  // rather than from the welcome, because a socket that greets and then
  // drops before saying anything is a link that never worked, and
  // clearing the count there holds the ladder at its first rung.
  const link =
    source.failures === 0 || source.state.kind !== "live"
      ? source
      : { ...source, failures: 0 };
  if (link.state.kind === "handshaking") {
    if ("welcome" in frame) {
      return welcomed(link, frame.welcome);
    }
    if ("refusal" in frame) {
      return refuse(link, frame.refusal);
    }
    // A server that streams or answers before welcoming is not speaking
    // this protocol; treat it as the mismatch it is.
    return refuse(link, outOfOrder(link.lang));
  }
  if (link.state.kind !== "live") {
    return [link, { kind: "nothing" }];
  }
  // A second greeting on a live link. It is the same disagreement as any
  // other out-of-order frame, and naming it is what the compiler's
  // exhaustiveness check asks for: silence here used to be the fallback
  // that made a frame this build does not handle invisible.
  if ("welcome" in frame) {
    return refuse(link, outOfOrder(link.lang));
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
    return refused(link, frame.refusal);
  }
  if ("lagged" in frame) {
    return [link, { kind: "lagged", from: frame.lagged.from, to: frame.lagged.to }];
  }
  return unhandled(link, frame);
}

// The compiler's check that every frame the generated schema admits has a
// branch above. A frame this build was not taught keeps its own type here
// and does not compile; what this replaces returned `nothing` for every
// frame, so forgetting one was a hole no build error named - the wire said
// a range had been skipped and the page folded it as silence.
function unhandled(link: Link, _frame: never): [Link, LinkAction] {
  return refuse(link, unreadable(link.lang));
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
    case "undecodable":
      return refuse(link, unreadable(link.lang));
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
