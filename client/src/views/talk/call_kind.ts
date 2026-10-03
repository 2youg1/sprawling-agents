// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The word a tool line opens with: what kind of thing the call did, read
// off what the tool was registered as (`Call.render`, then `Call.effect`,
// both from `kernel::ToolMeta` through the `tool_called` line), never off
// the tool's name. A tool added tomorrow is drawn by its registration the
// day it lands; a table of names here would be a second registry that
// learns of it later (client/Spec.lean §4-26).
//
// **The drawing intent comes first** because it is the more specific of
// the two: a terminal is an `exec` whatever boundary it crosses, and a
// diff is an `edit`. A call whose line carried neither - a tool the bench
// did not know, a line written before the keys existed - is named by the
// tool itself, which is the one honest word left.

import { Option, Schema } from "effect";

import type { Key } from "../../core/lang";
import type { Call } from "../../wire";

// What the subject cell of a line says. A send and a delegation read
// their own arguments (client D85); every other call reads its subject.
export type CallLine =
  | { readonly kind: "subject"; readonly subject: string }
  | { readonly kind: "worded"; readonly word: Key; readonly fills: Readonly<Record<string, string>> };

export type CallKind = { readonly kind: "registered"; readonly word: Key } | { readonly kind: "unregistered"; readonly tool: string };

export function kindOf(call: Call): CallKind {
  const render = call.render ?? null;
  if (render === "terminal") return { kind: "registered", word: "talk_kind_exec" };
  if (render === "signal") return { kind: "registered", word: Option.isSome(decoded(call.arguments, Pull)) ? "talk_kind_pull" : "talk_kind_send" };
  if (render === "delegate") return { kind: "registered", word: "talk_kind_delegate" };
  if (render !== null && render !== "generic") return { kind: "registered", word: "talk_kind_edit" };
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
  // A boundary the kernel adds fails to compile here until it has a word.
  const unnamed: never = effect;
  return unnamed;
}

// The subject cell. Arguments cut by the window, or not readable as the
// tool's own grammar, fall back to the recorded subject: the line keeps
// what the Ledger says rather than a guess at the missing half.
export function lineOf(call: Call): CallLine {
  const fallback: CallLine = { kind: "subject", subject: call.subject ?? "" };
  if (call.render === "signal") {
    return Option.match(decoded(call.arguments, Asked), {
      onNone: () => fallback,
      onSome: (sent): CallLine => ({ kind: "worded", word: "talk_send_line", fills: { to: sent.to, text: firstLine(sent.text) } }),
    });
  }
  if (call.render === "delegate") {
    return Option.match(decoded(call.arguments, Handed), {
      onNone: () => fallback,
      onSome: (handed): CallLine => ({ kind: "worded", word: "talk_delegate_line", fills: { room: handed.room, task: firstLine(handed.task) } }),
    });
  }
  return fallback;
}

// What the result cell says once the tool answered: what the tool itself
// reported, never where the letter landed, which is decided after the
// call (collab D17). `null` while the call runs and for every other tool.
export function outcomeOf(call: Call): Key | null {
  if (call.outcome !== "answered") return null;
  if (call.render === "signal") {
    return Option.match(decoded(call.output, Sent), {
      onNone: () => null,
      onSome: (sent): Key => (sent.waiting === undefined ? "talk_send_sent" : "talk_send_waiting"),
    });
  }
  if (call.render === "delegate") return Option.isSome(decoded(call.output, Started)) ? "talk_delegate_starts" : null;
  return null;
}

// The signal tool's own argument grammar (crates/collab/src/signal_tool.rs).
// wording-ok: an argument value the tool reads, never shown to a reader
const Pull = Schema.Struct({ action: Schema.Literal("pull") });
// wording-ok: an argument value the tool reads, never shown to a reader
const Asked = Schema.Struct({ action: Schema.Literal("send"), to: Schema.String, text: Schema.String });
const Handed = Schema.Struct({ room: Schema.String, task: Schema.String });
const Sent = Schema.Struct({ delivered: Schema.Boolean, waiting: Schema.optional(Schema.String) });
const Started = Schema.Struct({ room: Schema.String, starts: Schema.String });

function decoded<A>(output: Call["arguments"], schema: Schema.Decoder<A>): Option.Option<A> {
  if (output === null || output === undefined || output.cut > 0) return Option.none();
  return Schema.decodeUnknownOption(Schema.fromJsonString(schema))(output.head);
}

function firstLine(text: string): string {
  return text.split("\n", 1)[0] ?? "";
}
