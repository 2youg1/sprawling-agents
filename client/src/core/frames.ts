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
// per model token — and the full Effect decode costs several times the
// guard below, most of it parser machinery rather than checking. Those
// two frames are read with `JSON.parse` and a narrow guard whose every
// rule is taken from the generated schema: the event-kind set from the
// `EventKind` literals, and each identity value's pattern from the
// checks the schema itself carries on its string node. Every other
// frame, and a hot frame the guard does not accept, still goes through
// Effect, so the guard can only speed a frame up and never admit one the
// schema refuses.

import { Result, Schema, SchemaAST } from "effect";

import type { ClientFrame, Delta, EventRecord } from "../wire";
import { Address, B3Hash, EventKind, RunId, ServerFrame } from "../wire";

const readServerFrame = Schema.decodeUnknownResult(ServerFrame);

export function decodeFrame(text: string): ServerFrame | null {
  const parsed = Result.try((): unknown => JSON.parse(text));
  if (Result.isFailure(parsed)) return null;
  const value = parsed.success;
  if (isRecord(value)) {
    if ("event" in value && isEventRecord(value.event)) {
      return { event: value.event };
    }
    if ("delta" in value && isDelta(value.delta)) {
      return { delta: value.delta };
    }
  }
  return Result.getOrNull(readServerFrame(value));
}

// Every frame this client builds is serialisable: nothing in the type
// carries a credential, so `JSON.stringify` cannot fail on it.
export function encodeFrame(frame: ClientFrame): string {
  return JSON.stringify(frame);
}

// Every literal a union of literals admits, however the generator
// grouped them: one `Literal` per documented variant, one `Literals`
// node for the undocumented rest.
function literalsOf(ast: SchemaAST.AST): readonly unknown[] {
  if (SchemaAST.isLiteral(ast)) return [ast.literal];
  if (SchemaAST.isUnion(ast)) return ast.types.flatMap(literalsOf);
  return [];
}

const eventKinds: ReadonlySet<unknown> = new Set(literalsOf(EventKind.ast));

// The generated identity values are strings carrying their pattern as a
// check; running those checks directly skips the parser that wraps them.
function checkedString(ast: SchemaAST.AST): (u: unknown) => boolean {
  const checks = SchemaAST.isString(ast) ? ast.checks : undefined;
  if (checks === undefined) return () => false;
  return (u) => typeof u === "string" && checks.every((check) => passes(check, u, ast));
}

function passes(check: SchemaAST.Check<string>, u: string, ast: SchemaAST.AST): boolean {
  switch (check._tag) {
    case "Filter":
      return check.run(u, ast, {}) === undefined;
    case "FilterGroup":
      return check.checks.every((inner) => passes(inner, u, ast));
  }
}

const isRunIdText = checkedString(RunId.ast);
const isB3HashText = checkedString(B3Hash.ast);
const isAddressText = checkedString(Address.ast);

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
