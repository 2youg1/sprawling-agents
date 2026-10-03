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

export function lineOf(call: Call): CallLine {
  return { kind: "subject", subject: call.subject ?? "" };
}

export function outcomeOf(_call: Call): Key | null {
  return null;
}
