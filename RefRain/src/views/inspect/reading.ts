// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { Option, Schema } from "effect";

import { Locator } from "../../wire";
import type { Call, RoundsAnswer } from "../../wire";
import type { RightItem } from "./open.svelte";

export type Reading = "terminal" | "printed" | "diff" | "file" | "shot";

export type Region = "editor" | "terminal";

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

const ReadOne = Schema.Union([
  Schema.Struct({
    file_path: Schema.String,
    offset: Schema.optional(Schema.Int),
    limit: Schema.optional(Schema.Int),
  }),
  Schema.Struct({
    path: Schema.String,
    offset: Schema.optional(Schema.Int),
    limit: Schema.optional(Schema.Int),
  }),
]);

function readFromAsked(read: typeof ReadOne.Type): ReadFrom {
  if ("file_path" in read) return { path: read.file_path, line: Math.max(read.offset ?? 1, 1) };
  return { path: read.path, line: (read.offset ?? 0) + 1 };
}

const Picture = Schema.Struct({
  image: Locator,
  width: Schema.Int,
  height: Schema.Int,
  media_type: Schema.String,
});
export type Picture = typeof Picture.Type;

export interface ReadFrom {
  readonly path: string;

  readonly line: number;
}

export function readFrom(call: Call): ReadFrom | null {
  const asked = call.arguments;
  if (call.effect !== "read" || asked === null || asked === undefined || asked.cut > 0) return null;
  return Option.match(
    Schema.decodeUnknownOption(Schema.fromJsonString(ReadOne), { onExcessProperty: "error" })(
      asked.head,
    ),
    {
      onNone: () => null,
      onSome: readFromAsked,
    },
  );
}

export function pictureOf(call: Call): Picture | null {
  const head = call.output?.head;
  return head === undefined
    ? null
    : Option.getOrNull(Schema.decodeUnknownOption(Schema.fromJsonString(Picture))(head));
}

export function readingOf(call: Call): Reading {
  const render = call.render ?? "generic";
  if (render === "terminal") return "terminal";

  if (render === "signal" || render === "delegate") return "printed";
  if (render !== "generic") return "diff";
  if (pictureOf(call) !== null) return "shot";
  return readFrom(call) === null ? "printed" : "file";
}

export function followingIn(rounds: RoundsAnswer): readonly RightItem[] {
  const calls = rounds.turns.flatMap((turn) => turn.calls).reverse();
  const editor = calls.find(
    (call) => regionOf(readingOf(call)) === "editor" && call.outcome === "answered",
  );
  const terminal = calls.find(
    (call) => readingOf(call) === "terminal" && call.outcome !== "failed",
  );
  return [editor, terminal].flatMap((call): RightItem[] =>
    call === undefined ? [] : [{ kind: "call", run: rounds.run, at: call.at }],
  );
}
