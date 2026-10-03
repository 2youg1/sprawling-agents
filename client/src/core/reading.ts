// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// Reading one record's payload: the struct `kernel::event::record` wrote
// it from.
//
// **Why the client reads at all.** A record travels with `Payload`, an
// object whose values are unknown, because the event vocabulary is open
// on purpose - a ledger holds kinds written by builds other than this
// one. The structs that write a payload are the authority for which keys
// exist, which may be absent, and what each holds; this module is the
// fold's one reading of them, so a key is spelled in one place and a
// shape this build cannot read is answered rather than read as absence.
// (`core/probed.ts` owns the one payload with a shape of its own,
// `endpoint_probed`; it is a reader of the same family, not a second
// spelling of these keys.)
//
// **Absence and the wrong shape are different facts.** `RunStarted::task`
// is a `String` under `#[serde(default)]`, so a record that leaves the
// key out wrote the empty string. `ToolCalled::subject` is an `Option`,
// so leaving it out says this call names nothing of that kind - a state,
// not a failure. A key present in another shape is a wire this build was
// not taught: the reader answers the field it could not read, and
// `core/socket.ts` reports it the way a frame it cannot decode is
// reported.

import { Option, Schema } from "effect";

import { scopeOf } from "./scope";
import { RunPolicy, Waiting } from "../wire";
import type { EventRecord, HaltScope, ReplyEnd, Seq, TimeMs } from "../wire";

// One field of a payload, with the name that field is known by when this
// build cannot read it. `at` is null when it could.
interface Field<T> {
  readonly value: T;
  readonly at: string | null;
}

// How a field is named in what a person is told: the record's kind and
// the key, which is what they would find in the ledger.
function where(record: EventRecord, key: string): string {
  return `${record.kind}.${key}`;
}

// `String`, which the struct requires.
function required(record: EventRecord, key: string): Field<string | null> {
  const held = record.data[key];
  return typeof held === "string"
    ? { value: held, at: null }
    : { value: null, at: where(record, key) };
}

// `Option<String>`, with the serde default: absent is a state the record
// states rather than a failure.
function optional(record: EventRecord, key: string): Field<string | null> {
  const held = record.data[key];
  if (held === undefined || held === null) return { value: null, at: null };
  return typeof held === "string"
    ? { value: held, at: null }
    : { value: null, at: where(record, key) };
}

// `String` under `#[serde(default)]`: a record that leaves the key out
// wrote the empty string, because that is what serde reads it as.
function defaulted(record: EventRecord, key: string): Field<string> {
  const held = record.data[key];
  if (held === undefined) return { value: "", at: null };
  return typeof held === "string"
    ? { value: held, at: null }
    : { value: "", at: where(record, key) };
}

// One of a closed set of words, which the record's own writer chose.
function oneOf<T extends string>(
  record: EventRecord,
  key: string,
  words: readonly T[],
): Field<T | null> {
  const held = record.data[key];
  const found =
    typeof held === "string" ? words.find((word) => word === held) : undefined;
  return found === undefined
    ? { value: null, at: where(record, key) }
    : { value: found, at: null };
}

// The first of these fields this build could not read, or null when it
// read them all.
function first(...fields: readonly Field<unknown>[]): string | null {
  for (const held of fields) {
    if (held.at !== null) return held.at;
  }
  return null;
}

// `RunStarted::task` and `goal`: what the person asked for and what
// finishing looks like. The empty string is a run dispatched with
// nothing written down, which is what the struct's default reads an
// absent key as, so the two are one state rather than two.
export interface Opening {
  readonly task: string;
  readonly goal: string;
}

export function openingOf(record: EventRecord): [Opening, string | null] {
  const task = defaulted(record, "task");
  const goal = defaulted(record, "goal");
  return [{ task: task.value, goal: goal.value }, first(task, goal)];
}

// The two fields of `ToolCalled` this page folds: the tool the model
// asked for, and the one argument a person recognises the call by. The
// subject is read here and never derived from `args`, because the city
// already derived it once when the record was written.
export interface ToolCall {
  readonly name: string | null;
  readonly subject: string | null;
}

export function toolCall(record: EventRecord): [ToolCall, string | null] {
  const name = required(record, "name");
  const subject = optional(record, "subject");
  return [{ name: name.value, subject: subject.value }, first(name, subject)];
}

// `ModelCalled::model`: the model this call went to.
export function modelOf(record: EventRecord): [string | null, string | null] {
  const held = required(record, "model");
  return [held.value, first(held)];
}

// `RunFrozen::completion`, which names how the run ended. The words are
// `Completion::name`'s, and this reader passes them on unopened: a page
// that mapped them to an enum here would be a second place that decides
// which endings exist.
export function completionOf(record: EventRecord): [string | null, string | null] {
  const held = required(record, "completion");
  return [held.value, first(held)];
}

// `ApprovalItem::action_desc`: what an asking run waits to be allowed.
export function askOf(record: EventRecord): [string | null, string | null] {
  const held = required(record, "action_desc");
  return [held.value, first(held)];
}

// `collab::OpenRequest::branch`: the name a pull request goes by.
export function branchOf(record: EventRecord): [string | null, string | null] {
  const held = required(record, "branch");
  return [held.value, first(held)];
}

// `SignalWaitStarted::on` and `deadline_ms`, read as the wire's own
// `Waiting`: the deadline is a reading of the city's injected clock in
// epoch milliseconds, so it is already the wall-clock instant
// `RunSummary.waiting.until` carries (wire D34). A record either key of
// which this build cannot read names the first one that failed.
export function waitOf(record: EventRecord): [Waiting | null, string | null] {
  const read = Schema.decodeUnknownOption(Waiting)({ on: record.data.on, until: record.data.deadline_ms });
  if (Option.isSome(read)) return [read.value, null];
  const on = Schema.decodeUnknownOption(Waiting.fields.on)(record.data.on);
  return [null, where(record, Option.isSome(on) ? "deadline_ms" : "on")];
}

// The three words `kernel::event::record::WaitEnd` tags its arms with.
// Closed: a fourth is a wire this build was not taught, and a caller
// must then leave the run where it was.
const WAIT_ENDS: readonly ReplyEnd[] = ["reply", "timeout", "left"];

// `SignalWaitEnded::by`, an object tagged by `end`.
export function waitEndOf(record: EventRecord): [ReplyEnd | null, string | null] {
  const by = record.data.by;
  const tag = typeof by === "object" && by !== null && "end" in by ? by.end : undefined;
  const found = WAIT_ENDS.find((word) => word === tag);
  return found === undefined ? [null, where(record, "by.end")] : [found, null];
}

// The two words `kernel::event::record::Admittance` spells. Closed on
// both sides: a record carrying a third is a wire this build cannot
// read, not a release.
const ADMITTANCE = ["halted", "released"] as const;

// What one `CityHalted` record states: which scope stopped or started
// taking work, and which of the two it did. A null is a field this build
// could not read, and a caller must then change nothing - reading an
// unknown word as a release is how a stopped building looks free again.
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

// Where a session starts at an address: the room, and the sequence
// number of the line that began the newest stretch of it.
//
// `null` for every record that is not a session start, and for one that
// names no address - a session is a stretch of a room, and a line that
// does not say which room is not one the page can file. The caller
// keeps the newest: a page that reloads folds an older range after a
// newer one, and the newest start is what a stretch begins at.
export function sessionStart(record: EventRecord): SessionStart | null {
  if (record.kind !== "session_opened") return null;
  const addr = record.addr ?? null;
  return addr === null ? null : { addr, seq: record.seq };
}

// `run_policy_changed`: the room's run policy from the next safe point
// on (`kernel::event::record::RunPolicyChanged`). A record that is not
// one, names no room, or carries a policy this build cannot read
// answers null: the page then keeps the policy the run opened under.
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
