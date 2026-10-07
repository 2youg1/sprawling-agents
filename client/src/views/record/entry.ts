// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// One row of the record's timeline, decided (client D95: a seat, a look
// and this file): the time of day to the millisecond in UTC, the ledger
// position, what happened and where, every word in the person's
// language. A ledger record opens to the record as it was written, the
// chain's own fields included; a log line is one line and opens
// nothing. The day is written above the first row of each day.

import type { Lang } from "../../core/lang";
import { fill, say } from "../../core/lang";
import { isoInstant, isoTime } from "../../core/time";
import type { EventRecord, Seq } from "../../wire";
import { factsOf, whatHappened } from "./event";
import { LEVEL_NAMES } from "./timeline";
import type { Entry } from "./timeline";

// Spread on the record's row button: it opens and closes the record.
export interface OpenWire {
  readonly type: "button";
  readonly "aria-expanded": boolean;
  readonly onclick: () => void;
}

// A moment, written for a reader and stated for a machine.
export interface MomentLook {
  readonly text: string;
  readonly at: string;
}

export interface DayLook {
  readonly day: string;
  // The zone every time under it is written in.
  readonly zone: string;
}

export type RowLook =
  | {
      readonly kind: "record";
      readonly when: MomentLook;
      readonly seq: string;
      readonly what: string;
      readonly facts: string;
      readonly where: string;
      readonly open: boolean;
      readonly wire: OpenWire;
      readonly rawLabel: string;
      // The record as it was written; present only while it is open.
      readonly raw: string | undefined;
    }
  | {
      readonly kind: "log";
      // Absent for a line the page holds without a time.
      readonly when: MomentLook | undefined;
      readonly seq: string;
      readonly level: string;
      readonly line: string;
      readonly module: string;
    };

export interface EntryLook {
  readonly day: DayLook | undefined;
  readonly row: RowLook;
}

export interface EntryProps {
  readonly entry: Entry;
  // The day this entry opens, from `daysOf`; `null` when it opens none.
  readonly day: string | null;
  readonly open: boolean;
  readonly onToggle: (seq: Seq) => void;
}

export function lookOf(props: EntryProps, lang: Lang): EntryLook {
  return {
    day: props.day === null ? undefined : { day: props.day, zone: say(lang, "rec_utc") },
    row: rowOf(props, lang),
  };
}

function rowOf(props: EntryProps, lang: Lang): RowLook {
  const { entry } = props;
  switch (entry.kind) {
    case "record": {
      const { record } = entry;
      return {
        kind: "record",
        when: momentOf(record.t),
        seq: `#${String(record.seq)}`,
        what: say(lang, whatHappened(record.kind)),
        facts: factLine(record),
        where: record.addr ?? fill(say(lang, "rec_by"), { who: record.who }),
        open: props.open,
        wire: {
          type: "button",
          "aria-expanded": props.open,
          onclick: () => {
            props.onToggle(record.seq);
          },
        },
        rawLabel: say(lang, "rec_raw"),
        raw: props.open ? JSON.stringify(record, null, 2) : undefined,
      };
    }
    case "log":
      return {
        kind: "log",
        when: entry.t === null ? undefined : momentOf(entry.t),
        seq: `#${String(entry.line.seq)}`,
        level: say(lang, LEVEL_NAMES[entry.line.level]),
        line: entry.line.line,
        module: entry.line.module,
      };
  }
}

function momentOf(t: number): MomentLook {
  return { text: isoTime(t), at: isoInstant(t) };
}

function factLine(record: EventRecord): string {
  return factsOf(record.data)
    .map((fact) => `${fact.name}: ${fact.value}`)
    .join(" · ");
}
