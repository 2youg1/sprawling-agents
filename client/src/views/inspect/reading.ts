// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// How the inspector reads one call (client-SPEC 4-45). The tool's own
// registration is the authority on what a call is: `Call.render` says a
// call is a terminal or a diff, `Call.effect` that it only read
// (`kernel::ToolMeta`), and nothing here keeps a table of tool names. The
// two readings `render` does not name are told apart by the shape of what
// the call recorded, which is what each view goes on to read anyway: a
// picture the content store keeps, or one file read from a line.

import { Option, Schema } from "effect";

import { Locator } from "../../wire";
import type { Call, RoundsAnswer } from "../../wire";
import type { RightItem } from "./open.svelte";

// What the inspector draws for a call. `printed` is any other call: its
// tool, what it was asked, and what it answered.
export type Reading = "terminal" | "printed" | "diff" | "file" | "shot";

// The two regions of the right side: the editor above, the terminal
// below (7F).
export type Region = "editor" | "terminal";

// One tab of the inspector's strip: the item, the name it is shown by,
// and the region it opens in.
export interface Tab {
  readonly item: RightItem;
  readonly label: string;
  readonly region: Region;
}

export function regionOf(reading: Reading): Region {
  switch (reading) {
    case "terminal":
    case "printed":
      return "terminal";
    case "diff":
    case "file":
    case "shot":
      return "editor";
  }
}

// The arguments of a call that read one file from one line, and nothing
// else: a search also reads under a path, but it asks for a pattern, and
// an excess key refuses the decode.
const ReadOne = Schema.Struct({
  path: Schema.String,
  offset: Schema.optional(Schema.Int),
  limit: Schema.optional(Schema.Int),
});

// A screenshot as the browser tool records it: the picture is in the
// content store, and the record names it and its two sides.
const Picture = Schema.Struct({
  image: Locator,
  width: Schema.Int,
  height: Schema.Int,
  media_type: Schema.String,
});
export type Picture = typeof Picture.Type;

export interface ReadFrom {
  readonly path: string;
  // One-based, as an editor counts.
  readonly line: number;
}

export function readFrom(call: Call): ReadFrom | null {
  const asked = call.arguments;
  if (call.effect !== "read" || asked === null || asked === undefined || asked.cut > 0) return null;
  return Option.match(Schema.decodeUnknownOption(Schema.parseJson(ReadOne), { onExcessProperty: "error" })(asked.head), {
    onNone: () => null,
    onSome: (read) => ({ path: read.path, line: (read.offset ?? 0) + 1 }),
  });
}

export function pictureOf(call: Call): Picture | null {
  const head = call.output?.head;
  return head === undefined ? null : Option.getOrNull(Schema.decodeUnknownOption(Schema.parseJson(Picture))(head));
}

export function readingOf(call: Call): Reading {
  const render = call.render ?? "generic";
  if (render === "terminal") return "terminal";
  if (render !== "generic") return "diff";
  if (pictureOf(call) !== null) return "shot";
  return readFrom(call) === null ? "printed" : "file";
}

// What the right side follows while nobody has opened an item: the
// newest call of this run in each region, so a person watching a run sees
// the file it last touched above the command it last ran. A command still
// running is followed for its live tail; any other call is followed once
// it has answered something, and `printed` calls are not followed at all,
// because a run that only planned and delegated has nothing to inspect.
export function followingIn(rounds: RoundsAnswer): readonly RightItem[] {
  const calls = rounds.turns.flatMap((turn) => turn.calls).reverse();
  const editor = calls.find((call) => regionOf(readingOf(call)) === "editor" && call.outcome === "answered");
  const terminal = calls.find((call) => readingOf(call) === "terminal" && call.outcome !== "failed");
  return [editor, terminal].flatMap((call): RightItem[] => (call === undefined ? [] : [{ kind: "call", run: rounds.run, at: call.at }]));
}
