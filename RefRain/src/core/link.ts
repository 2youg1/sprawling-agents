// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type {
  Answered,
  AxError,
  ClientFrame,
  CloseMode,
  Delta,
  EventRecord,
  IdemKey,
  LiveOutput,
  LogLine,
  Sample,
  Seq,
  ServerFrame,
  Welcome,
} from "../wire";
import type { Key, Lang } from "./lang";
import { fill, say } from "./lang";
import { B3Hash, WIRE_HASH, WIRE_V } from "../wire";
import type { FrameFailure } from "./frames";

const SCHEMA: B3Hash = B3Hash.make(WIRE_HASH);

const LADDER_MS: readonly number[] = [250, 500, 1000, 2000, 5000, 10000];

export type LinkState =
  | { readonly kind: "idle" }
  | { readonly kind: "opening" }
  | { readonly kind: "handshaking" }
  | { readonly kind: "live"; readonly city: string | null }
  | { readonly kind: "backoff"; readonly attempt: number }
  | { readonly kind: "refused"; readonly error: AxError }
  | { readonly kind: "closed" };

export type LinkEvent =
  | { readonly kind: "opened"; readonly credential: string | null }
  | { readonly kind: "received"; readonly frame: ServerFrame }
  | { readonly kind: "undecodable"; readonly failure?: FrameFailure; readonly frame?: number }
  | { readonly kind: "closed" }
  | { readonly kind: "wait_elapsed" }
  | { readonly kind: "retry" }
  | { readonly kind: "closing"; readonly mode: CloseMode };

export type LinkAction =
  | { readonly kind: "nothing" }
  | { readonly kind: "open" }
  | { readonly kind: "send"; readonly frame: ClientFrame }
  | { readonly kind: "welcomed"; readonly welcome: Welcome }
  | { readonly kind: "deliver"; readonly event: EventRecord }
  | { readonly kind: "answered"; readonly answered: Answered }
  | { readonly kind: "accepted"; readonly request: IdemKey }
  | { readonly kind: "saying"; readonly delta: Delta }
  | { readonly kind: "logged"; readonly line: LogLine }
  | { readonly kind: "writing"; readonly piece: LiveOutput }
  | { readonly kind: "lagged"; readonly from: Seq; readonly to: Seq }
  | { readonly kind: "sampled"; readonly sample: Sample }
  | { readonly kind: "wait"; readonly ms: number }
  | { readonly kind: "report"; readonly error: AxError }
  | { readonly kind: "close" };

export interface Link {
  readonly state: LinkState;
  readonly lang: Lang;
  readonly closing: CloseMode | null;
  readonly failures: number;
}

export function newLink(lang: Lang): Link {
  return { state: { kind: "idle" }, lang, closing: null, failures: 0 };
}

export function isLive(link: Link): boolean {
  return link.state.kind === "live";
}

export function isStopped(link: Link): boolean {
  return link.state.kind === "refused" || link.state.kind === "closed";
}

export function backoffMs(attempt: number): number {
  const last = LADDER_MS.length - 1;
  return LADDER_MS[Math.min(Math.max(attempt, 0), last)] ?? 10000;
}

function refusal(lang: Lang, action: Key, subject: string, recovery: Key): AxError {
  return {
    code: "E_WIRE_MISMATCH",
    action: say(lang, action),
    subject,
    recovery: say(lang, recovery),
    nearby: [],
    retry: "no",
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

function unreadable(lang: Lang, event?: Extract<LinkEvent, { kind: "undecodable" }>): AxError {
  const failure = event?.failure;
  const error = refusal(
    lang,
    "wire_read_frame_action",
    fill(say(lang, "wire_frame_undecodable"), { ours: String(WIRE_V) }),
    "wire_reload_for_client",
  );
  if (failure === undefined) return error;
  const subject =
    failure.stage === "json"
      ? fill(say(lang, "wire_frame_json"), { frame: String(event?.frame), at: failure.at ?? "?" })
      : fill(say(lang, "wire_frame_schema"), {
          frame: String(event?.frame),
          path: failure.path,
          expected: failure.expected,
          actual: failure.actual,
        });
  return {
    ...error,
    subject: `${error.subject}. ${subject}`,
    nearby: [
      `stage = ${JSON.stringify(failure.stage)}`,
      `frame = ${String(event?.frame)}`,
      `wire_v = ${String(WIRE_V)}`,
      ...(failure.stage === "schema"
        ? [
            `path = ${JSON.stringify(failure.path)}`,
            `expected = ${JSON.stringify(failure.expected)}`,
            `actual_type = ${JSON.stringify(failure.actual)}`,
          ]
        : failure.at === null
          ? []
          : [`position = ${JSON.stringify(failure.at)}`]),
    ],
  };
}

export function unreadableRecord(lang: Lang, at: string): AxError {
  return refusal(
    lang,
    "wire_read_record_action",
    fill(say(lang, "wire_record_undecodable"), { at }),
    "wire_reload_for_client",
  );
}

export function unsentCommand(lang: Lang, verb: string): AxError {
  return {
    code: "E_TIMEOUT",
    action: say(lang, "link_send_action"),
    subject: verb,
    recovery: say(lang, "link_send_when_connected"),
    nearby: [],
    retry: "yes",
  };
}

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

function refused(link: Link, error: AxError): [Link, LinkAction] {
  return error.code === "E_WIRE_MISMATCH" ? refuse(link, error) : [link, { kind: "report", error }];
}

function received(source: Link, frame: ServerFrame): [Link, LinkAction] {
  const link =
    source.failures === 0 || source.state.kind !== "live" ? source : { ...source, failures: 0 };
  if (link.state.kind === "handshaking") {
    if ("welcome" in frame) {
      return welcomed(link, frame.welcome);
    }
    if ("refusal" in frame) {
      return refuse(link, frame.refusal);
    }
    return refuse(link, outOfOrder(link.lang));
  }
  if (link.state.kind !== "live") {
    return [link, { kind: "nothing" }];
  }
  if ("welcome" in frame) {
    return refuse(link, outOfOrder(link.lang));
  }
  if ("event" in frame) {
    return [link, { kind: "deliver", event: frame.event }];
  }
  if ("answered" in frame) {
    const { outcome } = frame.answered;
    return "refusal" in outcome && outcome.refusal.code === "E_WIRE_MISMATCH"
      ? refuse(link, outcome.refusal)
      : [link, { kind: "answered", answered: frame.answered }];
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
  if ("accepted" in frame) {
    return [link, { kind: "accepted", request: frame.accepted }];
  }
  if ("lagged" in frame) {
    return [link, { kind: "lagged", from: frame.lagged.from, to: frame.lagged.to }];
  }
  if ("output" in frame) {
    return [link, { kind: "writing", piece: frame.output }];
  }
  if ("monitor" in frame) {
    return [link, { kind: "sampled", sample: frame.monitor }];
  }
  return unhandled(link, frame);
}

function unhandled(link: Link, _frame: never): [Link, LinkAction] {
  return refuse(link, unreadable(link.lang));
}

export function advance(link: Link, event: LinkEvent): [Link, LinkAction] {
  const { state } = link;
  if (state.kind === "refused" || state.kind === "closed") {
    return event.kind === "retry"
      ? connect({ ...link, closing: null, failures: 0 })
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
            hello: { wire_v: WIRE_V, schema: SCHEMA, token: event.credential },
          },
        },
      ];
    case "received":
      return received(link, event.frame);
    case "undecodable":
      return refuse(link, unreadable(link.lang, event));
    case "closed":
      return link.closing === null
        ? retreat(link)
        : [{ ...link, state: { kind: "closed" } }, { kind: "nothing" }];
    case "closing":
      return [{ ...link, closing: event.mode }, { kind: "nothing" }];
    case "wait_elapsed":
      return state.kind === "backoff" ? connect(link) : [link, { kind: "nothing" }];
    case "retry":
      return connect({ ...link, failures: 0 });
  }
}
