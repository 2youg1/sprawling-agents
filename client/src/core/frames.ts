// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// Reading and writing frames. The generated `Schema` is the only
// authority on what a frame may hold: a frame this build cannot parse is
// `null`, and the link judges that as a wire mismatch rather than an
// outage, because reconnecting meets the same frame again.
//
// Event and delta frames are the hot path — one per ledger record, one
// per model token — and the full Effect decode costs about twenty
// microseconds each, most of it parser machinery rather than checking.
// Those two frames are read with `JSON.parse` and a narrow guard whose
// every rule is taken from the generated schema: the event-kind set from
// the `EventKind` literals, and each identity value's pattern from the
// refinement the schema itself carries. Every other frame, and a hot
// frame the guard does not accept, still goes through Effect, so the
// guard can only speed a frame up and never admit one the schema refuses.

import { Either, Option, Schema, SchemaAST } from "effect";

import type { ClientFrame, Delta, EventRecord } from "../wire";
import { Address, B3Hash, EventKind, RunId, ServerFrame } from "../wire";

const readServerFrame = Schema.decodeUnknownEither(ServerFrame);

export function decodeFrame(text: string): ServerFrame | null {
  const parsed = Either.try((): unknown => JSON.parse(text));
  if (Either.isLeft(parsed)) return null;
  const value = parsed.right;
  if (isRecord(value)) {
    if ("event" in value && isEventRecord(value.event)) {
      return { event: value.event };
    }
    if ("delta" in value && isDelta(value.delta)) {
      return { delta: value.delta };
    }
  }
  const read = readServerFrame(value);
  return Either.isRight(read) ? read.right : null;
}

// Every frame this client builds is serialisable: nothing in the type
// carries a credential, so `JSON.stringify` cannot fail on it.
export function encodeFrame(frame: ClientFrame): string {
  return JSON.stringify(frame);
}

const eventKinds: ReadonlySet<unknown> = new Set(
  EventKind.members.flatMap((member) => member.literals),
);

// The generated identity values are string refinements; their filter is
// the pattern the Rust side declared, and calling it directly skips the
// parser that wraps it.
function refinedString(ast: SchemaAST.AST): (u: unknown) => boolean {
  if (!SchemaAST.isRefinement(ast)) return () => false;
  return (u) => typeof u === "string" && Option.isNone(ast.filter(u, {}, ast));
}

const isRunIdText = refinedString(RunId.ast);
const isB3HashText = refinedString(B3Hash.ast);
const isAddressText = refinedString(Address.ast);

function isRecord(u: unknown): u is Record<string, unknown> {
  return typeof u === "object" && u !== null && !Array.isArray(u);
}

function isInt(u: unknown): boolean {
  return Number.isSafeInteger(u);
}

function isEventRecord(u: unknown): u is EventRecord {
  return (
    isRecord(u) &&
    eventKinds.has(u.kind) &&
    isInt(u.seq) &&
    isInt(u.t) &&
    isInt(u.v) &&
    typeof u.who === "string" &&
    isRecord(u.data) &&
    isRunIdText(u.run) &&
    isB3HashText(u.prev) &&
    (u.addr === undefined || u.addr === null || isAddressText(u.addr)) &&
    (u.ig === undefined || typeof u.ig === "boolean")
  );
}

function isDelta(u: unknown): u is Delta {
  if (!isRecord(u) || !isRunIdText(u.run)) return false;
  const increment = u.increment;
  return (
    isRecord(increment) &&
    (typeof increment.said === "string" ||
      typeof increment.thought === "string")
  );
}
