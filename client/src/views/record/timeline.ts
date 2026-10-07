// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The record's one timeline (refrain S7.6 6c): the ledger's records and
// the process log's lines in one column, newest first, each line where
// the ledger stood when it was written. A log line carries the ledger
// position beside it, so the two sources share one axis without a
// clock comparison between two machines' worth of timestamps.
//
// The two sources are kept differently, and the timeline says so rather
// than pretend otherwise. The ledger is history and is read a page at a
// time; the log is a window the page has held since it opened, and a
// line that scrolled out of it is gone. Read with both sources, a page
// of the ledger shows the log lines written while that page's records
// were - the head page also every line written since.

import type { Key } from "../../core/lang";
import { isoDay } from "../../core/time";
import type { EventRecord, LogLevel, LogLine, RunId, Seq } from "../../wire";

// Which source the timeline reads. `log` is what `#/record/log` named
// when the process log was a lens of its own, and the address still
// opens on it.
export type Source = "every" | "ledger" | "log";

export const SOURCES: readonly Source[] = ["every", "ledger", "log"];

// The five levels `docs/logging.md` names, in its order, and the word
// for each: the level filter offers them and a log row is marked with one.
export const LEVELS: readonly LogLevel[] = ["refuse", "effect", "decide", "trace", "wire"];

export const LEVEL_NAMES: Record<LogLevel, Key> = {
  refuse: "log_refuse",
  effect: "log_effect",
  decide: "log_decide",
  trace: "log_trace",
  wire: "log_wire",
};

export type Entry =
  | { readonly kind: "record"; readonly key: string; readonly seq: Seq; readonly t: number; readonly record: EventRecord }
  | { readonly kind: "log"; readonly key: string; readonly seq: Seq; readonly t: number | null; readonly line: LogLine };

// What narrows the column besides the source: one run, one log level,
// one module. `null` is every one of them. A level or a module names a
// property only log lines have, so either one leaves no ledger record.
export interface Narrowing {
  readonly run: RunId | null;
  readonly level: LogLevel | null;
  readonly module: string | null;
}

export const WIDE: Narrowing = { run: null, level: null, module: null };

// The ledger page the timeline is reading: its records, oldest first as
// the answer gives them, and whether it is the page at the head.
export interface Page {
  readonly records: readonly EventRecord[];
  readonly head: boolean;
}

export function entriesOf(page: Page, lines: readonly LogLine[], source: Source, narrowing: Narrowing): readonly Entry[] {
  const records = source === "log" ? [] : page.records.filter((record) => recordKept(record, narrowing));
  const shown = source === "ledger" ? [] : lines.filter((line) => lineKept(line, narrowing) && within(page, source, line.seq));
  const entries: Entry[] = [
    ...records.map((record): Entry => ({ kind: "record", key: `r${String(record.seq)}`, seq: record.seq, t: record.t, record })),
    ...shown.map((line, at): Entry => ({ kind: "log", key: `l${String(line.seq)}-${String(at)}`, seq: line.seq, t: line.t ?? null, line })),
  ];
  // Newest first. A line written at ledger position n was written after
  // record n, so at one position the line stands above the record.
  return entries.sort((a, b) => b.seq - a.seq || order(a) - order(b));
}

// The day written above each entry that opens one, in UTC, and `null`
// above every other: the day is written once, above the first row of
// each day. A log line the page holds without a time opens no day and
// does not end one, so the next timed entry is read against the last
// timed entry above it.
export function daysOf(entries: readonly Entry[]): readonly (string | null)[] {
  let above: string | null = null;
  return entries.map((entry) => {
    if (entry.t === null) return null;
    const day = isoDay(entry.t);
    const opens = day === above ? null : day;
    above = day;
    return opens;
  });
}

function order(entry: Entry): number {
  return entry.kind === "log" ? 0 : 1;
}

function recordKept(record: EventRecord, narrowing: Narrowing): boolean {
  return narrowing.level === null && narrowing.module === null && (narrowing.run === null || record.run === narrowing.run);
}

function lineKept(line: LogLine, narrowing: Narrowing): boolean {
  return (
    (narrowing.run === null || line.run === narrowing.run) &&
    (narrowing.level === null || line.level === narrowing.level) &&
    (narrowing.module === null || line.module === narrowing.module)
  );
}

// Whether a log line falls on the ledger page being read. Read alone,
// the log is its own window and every line in it is shown.
function within(page: Page, source: Source, seq: Seq): boolean {
  if (source === "log") return true;
  const oldest = page.records[0]?.seq;
  const newest = page.records.at(-1)?.seq;
  if (oldest === undefined || newest === undefined) return page.head;
  return seq >= oldest && (page.head || seq <= newest);
}
