// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { Result, Schema, SchemaAST, SchemaIssue } from "effect";

import type { ClientFrame, Delta, EventRecord } from "../wire";
import { Address, B3Hash, EventKind, RunId, ServerFrame } from "../wire";

const readServerFrame = Schema.decodeUnknownResult(ServerFrame, { reportInput: false });
const fieldName = /^[a-z][a-z\d_]{0,39}$/i;

interface FieldIssue {
  readonly keys: readonly PropertyKey[];
  readonly expected: string;
}

function fieldIssue(issue: SchemaIssue.Issue, keys: readonly PropertyKey[] = []): FieldIssue {
  switch (issue._tag) {
    case "Pointer":
      return fieldIssue(issue.issue, [...keys, ...issue.path]);
    case "Filter": {
      const expected = issue.filter.annotations?.expected;
      return typeof expected === "string" ? { keys, expected } : fieldIssue(issue.issue, keys);
    }
    case "Encoding":
      return fieldIssue(issue.issue, keys);
    case "Composite":
      return fieldIssue(issue.issues[0], keys);
    case "AnyOf": {
      const candidates = issue.issues.map((candidate) => fieldIssue(candidate, keys));
      return (
        candidates.find((candidate) => candidate.keys.length > keys.length + 1) ??
        candidates[0] ?? { keys, expected: "ServerFrame" }
      );
    }
    case "Forbidden":
    case "InvalidType":
    case "InvalidValue":
    case "MissingKey":
    case "OneOf":
    case "UnexpectedKey":
      return { keys, expected: SchemaIssue.defaultLeafHook(issue) };
  }
}

export function decodeFrame(text: string): ServerFrame | null {
  const decoded = inspectFrame(text);
  return Result.getOrNull(decoded);
}

export type FrameFailure =
  | { readonly stage: "json"; readonly at: string | null }
  | {
      readonly stage: "schema";
      readonly path: string;
      readonly expected: string;
      readonly actual: string;
    };

export function inspectFrame(text: string): Result.Result<ServerFrame, FrameFailure> {
  const parsed = Result.try((): unknown => JSON.parse(text));
  if (Result.isFailure(parsed)) {
    const reason = parsed.failure instanceof Error ? parsed.failure.message : "";
    const position = /\b(?:position \d+|line \d+ column \d+)\b/i.exec(reason)?.[0];
    const end = /unexpected end/i.test(reason) ? String(text.length) : null;
    return Result.fail({ stage: "json", at: position ?? end });
  }
  const value = parsed.success;
  if (isRecord(value)) {
    if ("event" in value && isEventRecord(value.event)) {
      return Result.succeed({ event: value.event });
    }
    if ("delta" in value && isDelta(value.delta)) {
      return Result.succeed({ delta: value.delta });
    }
  }
  const decoded = readServerFrame(value);
  if (Result.isSuccess(decoded)) return Result.succeed(decoded.success);
  const { keys, expected } = fieldIssue(decoded.failure.issue);
  const actual = keys.reduce<unknown>((held, key) => {
    if (isRecord(held) && Object.hasOwn(held, key)) return held[String(key)];
    if (Array.isArray(held) && typeof key === "number") return held[key];
    return undefined;
  }, value);
  return Result.fail({
    stage: "schema",
    path:
      keys
        .map((key) => {
          if (typeof key === "number") return String(key);
          return typeof key === "string" && fieldName.test(key) ? key : "[key]";
        })
        .join(".") || "$",
    expected,
    actual: actual === null ? "null" : Array.isArray(actual) ? "array" : typeof actual,
  });
}

export function encodeFrame(frame: ClientFrame): string {
  return JSON.stringify(frame);
}

function literalsOf(ast: SchemaAST.AST): readonly unknown[] {
  if (SchemaAST.isLiteral(ast)) return [ast.literal];
  if (SchemaAST.isUnion(ast)) return ast.types.flatMap(literalsOf);
  return [];
}

const eventKinds: ReadonlySet<unknown> = new Set(literalsOf(EventKind.ast));

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
    (typeof increment.said === "string" || typeof increment.thought === "string")
  );
}
