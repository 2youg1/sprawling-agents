// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { Key } from "../../core/lang";
import { ymd } from "../../core/time";
import type { EventRecord, LogLevel, LogLine, RunId, Seq } from "../../wire";

export type Source = "every" | "ledger" | "log";

export const SOURCES: readonly Source[] = ["every", "ledger", "log"];

export const LEVELS: readonly LogLevel[] = ["refuse", "effect", "decide", "trace", "wire"];

export const LEVEL_NAMES: Record<LogLevel, Key> = {
  refuse: "log_refuse",
  effect: "log_effect",
  decide: "log_decide",
  trace: "log_trace",
  wire: "log_wire",
};

export type Entry =
  | {
      readonly kind: "record";
      readonly key: string;
      readonly seq: Seq;
      readonly t: number;
      readonly record: EventRecord;
    }
  | {
      readonly kind: "log";
      readonly key: string;
      readonly seq: Seq;
      readonly t: number | null;
      readonly line: LogLine;
    };

export interface Narrowing {
  readonly run: RunId | null;
  readonly level: LogLevel | null;
  readonly module: string | null;
}

export const WIDE: Narrowing = { run: null, level: null, module: null };

export interface Page {
  readonly records: readonly EventRecord[];
  readonly head: boolean;
}

export function entriesOf(
  page: Page,
  lines: readonly LogLine[],
  source: Source,
  narrowing: Narrowing,
): readonly Entry[] {
  const records =
    source === "log" ? [] : page.records.filter((record) => recordKept(record, narrowing));
  const shown =
    source === "ledger"
      ? []
      : lines.filter((line) => lineKept(line, narrowing) && within(page, source, line.seq));
  const entries: Entry[] = [
    ...records.map((record): Entry => ({
      kind: "record",
      key: `r${String(record.seq)}`,
      seq: record.seq,
      t: record.t,
      record,
    })),
    ...shown.map((line, at): Entry => ({
      kind: "log",
      key: `l${String(line.seq)}-${String(at)}`,
      seq: line.seq,
      t: line.t ?? null,
      line,
    })),
  ];
  return entries.sort((a, b) => b.seq - a.seq || order(a) - order(b));
}

export function daysOf(entries: readonly Entry[]): readonly (string | null)[] {
  let above: string | null = null;
  return entries.map((entry) => {
    if (entry.t === null) return null;
    const day = ymd(entry.t);
    const opens = day === above ? null : day;
    above = day;
    return opens;
  });
}

function order(entry: Entry): number {
  return entry.kind === "log" ? 0 : 1;
}

function recordKept(record: EventRecord, narrowing: Narrowing): boolean {
  return (
    narrowing.level === null &&
    narrowing.module === null &&
    (narrowing.run === null || record.run === narrowing.run)
  );
}

function lineKept(line: LogLine, narrowing: Narrowing): boolean {
  return (
    (narrowing.run === null || line.run === narrowing.run) &&
    (narrowing.level === null || line.level === narrowing.level) &&
    (narrowing.module === null || line.module === narrowing.module)
  );
}

function within(page: Page, source: Source, seq: Seq): boolean {
  if (source === "log") return true;
  const oldest = page.records[0]?.seq;
  const newest = page.records.at(-1)?.seq;
  if (oldest === undefined || newest === undefined) return page.head;
  return seq >= oldest && (page.head || seq <= newest);
}
