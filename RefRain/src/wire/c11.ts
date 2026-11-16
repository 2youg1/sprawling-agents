// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { Schema } from "effect";

import {
  Address,
  B3Hash,
  TimeMs,
} from "./c00";

import {
  RunId,
  Seq,
} from "./c01";

import {
  EventRecord,
} from "./c03";

import {
  AxError,
} from "./c04";

import {
  Answered,
} from "./c08";

import {
  Ask,
} from "./c09";

import {
  Command,
  Hello,
  Monitoring,
} from "./c10";

export const ClientFrame = Schema.Union([
  Schema.Struct({
    hello: Hello,
  }),
  Schema.Struct({
    command: Command,
  }),
  Schema.Struct({
    ask: Ask,
  }),
  Schema.Struct({
    monitor: Monitoring,
  }),
]).annotate({ identifier: "ClientFrame" });
export type ClientFrame = typeof ClientFrame.Type;

export const Increment = Schema.Union([
  Schema.Struct({
    said: Schema.String,
  }),
  Schema.Struct({
    thought: Schema.String,
  }),
]).annotate({ identifier: "Increment" });
export type Increment = typeof Increment.Type;

export const Delta = Schema.Struct({
  increment: Increment,
  run: RunId,
}).annotate({ identifier: "Delta" });
export type Delta = typeof Delta.Type;

export const Lagged = Schema.Struct({
  from: Seq,
  to: Seq,
}).annotate({ identifier: "Lagged" });
export type Lagged = typeof Lagged.Type;

export const OutputStream = Schema.Literals(["out", "err"]).annotate({ identifier: "OutputStream" });
export type OutputStream = typeof OutputStream.Type;

export const LiveOutput = Schema.Struct({
  run: RunId,
  stream: OutputStream,
  text: Schema.String,
}).annotate({ identifier: "LiveOutput" });
export type LiveOutput = typeof LiveOutput.Type;

export const LogLevel = Schema.Literals(["refuse", "effect", "decide", "trace", "wire"]).annotate({ identifier: "LogLevel" });
export type LogLevel = typeof LogLevel.Type;

export const LogLine = Schema.Struct({
  level: LogLevel,
  line: Schema.String,
  module: Schema.String,
  run: Schema.optional(Schema.NullOr(RunId)),
  seq: Seq,
  t: Schema.optional(Schema.NullOr(TimeMs)),
}).annotate({ identifier: "LogLine" });
export type LogLine = typeof LogLine.Type;

export const Sample = Schema.Struct({
  beat_ms: Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)),
  core_cpu_permille: Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)),
  core_private_bytes: Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)),
  core_read_bytes: Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)),
  core_working_set_bytes: Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)),
  core_written_bytes: Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)),
  durable_lag: Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)),
  event_to_screen_p50_nanos: Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)),
  ledger_queue_depth: Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)),
  machine_available_bytes: Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)),
  machine_cpu_permille: Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)),
  queued_runs: Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)),
  read_nanos: Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)),
  relay_p50_nanos: Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)),
  view_backlog: Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)),
  volume_free_bytes: Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)),
}).annotate({ identifier: "Sample" });
export type Sample = typeof Sample.Type;

export const Welcome = Schema.Struct({
  city: Schema.optional(Schema.NullOr(Address)),
  epoch: Schema.optional(Schema.NullOr(B3Hash)),
  resume_from: Schema.optional(Schema.NullOr(Seq)),
  schema: B3Hash,
  wire_v: Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)),
}).annotate({ identifier: "Welcome" });
export type Welcome = typeof Welcome.Type;

export const ServerFrame = Schema.Union([
  Schema.Struct({
    welcome: Welcome,
  }),
  Schema.Struct({
    event: EventRecord,
  }),
  Schema.Struct({
    answered: Answered,
  }),
  Schema.Struct({
    refusal: AxError,
  }),
  Schema.Struct({
    delta: Delta,
  }),
  Schema.Struct({
    log: LogLine,
  }),
  Schema.Struct({
    lagged: Lagged,
  }),
  Schema.Struct({
    output: LiveOutput,
  }),
  Schema.Struct({
    monitor: Sample,
  }),
]).annotate({ identifier: "ServerFrame" });
export type ServerFrame = typeof ServerFrame.Type;

