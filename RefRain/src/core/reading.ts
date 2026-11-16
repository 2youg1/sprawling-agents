// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { Option, Schema } from "effect";

import { scopeOf } from "./scope";
import { RunPolicy, Waiting } from "../wire";
import type { EventRecord, HaltScope, ReplyEnd, Seq, TimeMs } from "../wire";

interface Field<T> {
  readonly value: T;
  readonly at: string | null;
}

function where(record: EventRecord, key: string): string {
  return `${record.kind}.${key}`;
}

function required(record: EventRecord, key: string): Field<string | null> {
  const held = record.data[key];
  return typeof held === "string"
    ? { value: held, at: null }
    : { value: null, at: where(record, key) };
}

function optional(record: EventRecord, key: string): Field<string | null> {
  const held = record.data[key];
  if (held === undefined || held === null) return { value: null, at: null };
  return typeof held === "string"
    ? { value: held, at: null }
    : { value: null, at: where(record, key) };
}

function defaulted(record: EventRecord, key: string): Field<string> {
  const held = record.data[key];
  if (held === undefined) return { value: "", at: null };
  return typeof held === "string"
    ? { value: held, at: null }
    : { value: "", at: where(record, key) };
}

function oneOf<T extends string>(
  record: EventRecord,
  key: string,
  words: readonly T[],
): Field<T | null> {
  const held = record.data[key];
  const found = typeof held === "string" ? words.find((word) => word === held) : undefined;
  return found === undefined ? { value: null, at: where(record, key) } : { value: found, at: null };
}

function first(...fields: readonly Field<unknown>[]): string | null {
  for (const held of fields) {
    if (held.at !== null) return held.at;
  }
  return null;
}

export interface Opening {
  readonly task: string;
  readonly goal: string;
}

export function openingOf(record: EventRecord): [Opening, string | null] {
  const task = defaulted(record, "task");
  const goal = defaulted(record, "goal");
  return [{ task: task.value, goal: goal.value }, first(task, goal)];
}

export interface ToolCall {
  readonly name: string | null;
  readonly subject: string | null;
}

export function toolCall(record: EventRecord): [ToolCall, string | null] {
  const name = required(record, "name");
  const subject = optional(record, "subject");
  return [{ name: name.value, subject: subject.value }, first(name, subject)];
}

export function modelOf(record: EventRecord): [string | null, string | null] {
  const held = required(record, "model");
  return [held.value, first(held)];
}

export function completionOf(record: EventRecord): [string | null, string | null] {
  const held = required(record, "completion");
  return [held.value, first(held)];
}

export function askOf(record: EventRecord): [string | null, string | null] {
  const held = required(record, "action_desc");
  return [held.value, first(held)];
}

export function branchOf(record: EventRecord): [string | null, string | null] {
  const held = required(record, "branch");
  return [held.value, first(held)];
}

export function waitOf(record: EventRecord): [Waiting | null, string | null] {
  const read = Schema.decodeUnknownOption(Waiting)({
    on: record.data.on,
    until: record.data.deadline_ms,
  });
  if (Option.isSome(read)) return [read.value, null];
  const on = Schema.decodeUnknownOption(Waiting.fields.on)(record.data.on);
  return [null, where(record, Option.isSome(on) ? "deadline_ms" : "on")];
}

const WAIT_ENDS: readonly ReplyEnd[] = ["reply", "timeout", "left"];

export function waitEndOf(record: EventRecord): [ReplyEnd | null, string | null] {
  const by = record.data.by;
  const tag = typeof by === "object" && by !== null && "end" in by ? by.end : undefined;
  const found = WAIT_ENDS.find((word) => word === tag);
  return found === undefined ? [null, where(record, "by.end")] : [found, null];
}

const ADMITTANCE = ["halted", "released"] as const;

export interface Halt {
  readonly scope: HaltScope | null;
  readonly state: "halted" | "released" | null;
}

export function haltOf(record: EventRecord): [Halt, string | null] {
  const held = required(record, "scope");
  const state = oneOf(record, "state", ADMITTANCE);
  const scope = scopeOf(held.value);
  if (held.value !== null && scope === null) {
    return [{ scope: null, state: state.value }, where(record, "scope")];
  }
  return [{ scope, state: state.value }, first(held, state)];
}

export function sessionStart(record: EventRecord): SessionStart | null {
  if (record.kind !== "session_opened") return null;
  const addr = record.addr ?? null;
  return addr === null ? null : { addr, seq: record.seq };
}

export function policyChange(record: EventRecord): PolicyChange | null {
  if (record.kind !== "run_policy_changed") return null;
  const addr = record.addr ?? null;
  const policy = Option.getOrNull(Schema.decodeUnknownOption(RunPolicy)(record.data.policy));
  return addr === null || policy === null ? null : { addr, seq: record.seq, at: record.t, policy };
}

export interface PolicyChange {
  readonly addr: string;
  readonly seq: Seq;
  readonly at: TimeMs;
  readonly policy: RunPolicy;
}

export interface SessionStart {
  readonly addr: string;
  readonly seq: Seq;
}
