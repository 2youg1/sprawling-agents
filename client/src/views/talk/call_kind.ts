// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { Option, Schema } from "effect";

import type { Key } from "../../core/lang";
import type { Call, Landing } from "../../wire";

export type CallLine =
  | { readonly kind: "subject"; readonly subject: string }
  | {
      readonly kind: "worded";
      readonly word: Key;
      readonly fills: Readonly<Record<string, string>>;
    };

export type CallKind =
  | { readonly kind: "registered"; readonly word: Key }
  | { readonly kind: "unregistered"; readonly tool: string };

export function kindOf(call: Call): CallKind {
  const render = call.render ?? null;
  if (render === "terminal") return { kind: "registered", word: "talk_kind_exec" };
  if (render === "signal")
    return {
      kind: "registered",
      word: Option.isSome(decoded(call.arguments, Pull)) ? "talk_kind_pull" : "talk_kind_send",
    };
  if (render === "delegate") return { kind: "registered", word: "talk_kind_delegate" };
  if (render !== null && render !== "generic")
    return { kind: "registered", word: "talk_kind_edit" };
  const effect = call.effect ?? null;
  if (effect === null) return { kind: "unregistered", tool: call.tool };
  if (effect === "read") return { kind: "registered", word: "talk_kind_read" };
  if (effect === "spend") return { kind: "registered", word: "talk_kind_spend" };
  if (effect === "egress") return { kind: "registered", word: "talk_kind_egress" };
  if (effect === "spawn") return { kind: "registered", word: "talk_kind_spawn" };
  if (effect === "govern") return { kind: "registered", word: "talk_kind_govern" };
  if ("write" in effect) return { kind: "registered", word: "talk_kind_write" };
  if ("connector" in effect) return { kind: "registered", word: "talk_kind_connector" };
  if ("attach_user_browser" in effect) return { kind: "registered", word: "talk_kind_browser" };

  const unnamed: never = effect;
  return unnamed;
}

export function lineOf(call: Call): CallLine {
  const fallback: CallLine = { kind: "subject", subject: call.subject ?? "" };
  if (call.render === "signal") {
    return Option.match(decoded(call.arguments, Asked), {
      onNone: () => fallback,
      onSome: (sent): CallLine => ({
        kind: "worded",
        word: "talk_send_line",
        fills: { to: sent.to, text: firstLine(sent.text) },
      }),
    });
  }
  if (call.render === "delegate") {
    return Option.match(decoded(call.arguments, Handed), {
      onNone: () => fallback,
      onSome: (handed): CallLine => ({
        kind: "worded",
        word: "talk_delegate_line",
        fills: { room: handed.room, task: firstLine(handed.task) },
      }),
    });
  }
  return fallback;
}

export function outcomeOf(call: Call): Key | null {
  const landing =
    call.render === "signal" && call.outcome !== "failed" ? (call.landing ?? null) : null;
  if (landing !== null) return LANDED[landing];
  if (call.outcome !== "answered") return null;
  if (call.render === "signal") {
    return Option.match(decoded(call.output, Sent), {
      onNone: () => null,
      onSome: (sent): Key => (sent.waiting === undefined ? "talk_send_sent" : "talk_send_waiting"),
    });
  }
  if (call.render === "delegate")
    return Option.isSome(decoded(call.output, Started)) ? "talk_delegate_starts" : null;
  return null;
}

const LANDED: Readonly<Record<Landing, Key>> = {
  delivered: "talk_landed_delivered",
  queued: "talk_landed_queued",
  knocked: "talk_landed_knocked",
};

// wording-ok: an argument value the tool reads, never shown to a reader
const Pull = Schema.Struct({ action: Schema.Literal("pull") });
// wording-ok: an argument value the tool reads, never shown to a reader
const Asked = Schema.Struct({
  action: Schema.Literal("send"),
  to: Schema.String,
  text: Schema.String,
});
const Handed = Schema.Struct({ room: Schema.String, task: Schema.String });

const Refused = Schema.Struct({ code: Schema.String, subject: Schema.String });
const Sent = Schema.Struct({ delivered: Schema.Boolean, waiting: Schema.optional(Schema.String) });
const Started = Schema.Struct({ room: Schema.String, starts: Schema.String });

export type Failure =
  | { readonly kind: "refused"; readonly code: string; readonly subject: string }
  | { readonly kind: "said"; readonly text: string }
  | { readonly kind: "silent" };

export function failureOf(call: Call): Failure {
  const refused = decoded(call.output, Refused);
  if (Option.isSome(refused))
    return { kind: "refused", code: refused.value.code, subject: refused.value.subject };
  const text =
    (call.output?.head ?? "")
      .split("\n")
      .map((each) => each.trim())
      .find((each) => each !== "") ?? "";
  if (text === "") return { kind: "silent" };
  const kept = Array.from(text);
  return {
    kind: "said",
    text: kept.length > SAID_CHARS ? `${kept.slice(0, SAID_CHARS).join("").trimEnd()}…` : text,
  };
}

const SAID_CHARS = 160;

function decoded<A>(output: Call["arguments"], schema: Schema.Decoder<A>): Option.Option<A> {
  if (output === null || output === undefined || output.cut > 0) return Option.none();
  return Schema.decodeUnknownOption(Schema.fromJsonString(schema))(output.head);
}

function firstLine(text: string): string {
  return text.split("\n", 1)[0] ?? "";
}
