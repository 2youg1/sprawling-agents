// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { Answer, ArchiveHit, DiscardLine, EventRecord, LogLine, Query } from "../../wire";
import { Address, B3Hash, Locator, RunId, Seq, TimeMs } from "../../wire";
import type { Entry } from "../record/timeline";

const RUN = RunId.make("3f2a9c4e-0d1b-4e7a-9a55-1c2b3d4e5f60");
const NOON = Date.UTC(2026, 9, 2, 12);

const RECORD: EventRecord = {
  run: RUN,
  seq: Seq.make(44),
  kind: "checkpoint_committed",
  t: TimeMs.make(NOON + 1_204),
  who: "hall/mayor",
  addr: Address.make("shop/notes"),
  prev: B3Hash.make("0".repeat(64)),
  v: 1,
  data: { files: 2 },
};
const STARTED: EventRecord = {
  ...RECORD,
  seq: Seq.make(40),
  kind: "run_started",
  t: TimeMs.make(NOON - 2_412),
  addr: null,
  data: { task: "tidy the release notes" },
};
const UNTIMED: LogLine = {
  seq: Seq.make(44),
  t: null,
  level: "effect",
  module: "memory",
  line: "commit 9c41e07 on shop/notes",
  run: RUN,
};

const ENTRIES: readonly {
  readonly entry: Entry;
  readonly day: string | null;
  readonly open: boolean;
}[] = [
  {
    entry: { kind: "log", key: "l44", seq: UNTIMED.seq, t: null, line: UNTIMED },
    day: null,
    open: false,
  },
  {
    entry: { kind: "record", key: "r44", seq: RECORD.seq, t: RECORD.t, record: RECORD },
    day: "2026-10-02",
    open: true,
  },
  {
    entry: { kind: "record", key: "r40", seq: STARTED.seq, t: STARTED.t, record: STARTED },
    day: null,
    open: false,
  },
];

const HITS: readonly ArchiveHit[] = [
  {
    day: 20_363,
    kind: "memo",
    building: Address.make("shop/notes"),
    subject: "why the changelog keeps one entry per card",
  },
  {
    day: 20_361,
    kind: "plan",
    building: Address.make("shop/checkout"),
    subject: "move the receipt printer behind the payment step",
  },
  {
    day: 20_354,
    kind: "spec",
    building: Address.make("hall/registry"),
    subject: "every resident names the building it was placed in",
  },
];

const DISCARDS: readonly DiscardLine[] = [
  {
    at: TimeMs.make(NOON - 3_600_000),
    path: "shop/notes/draft-notes.md",
    restoration: { tracked: Locator.make("9c41e07:shop/notes/draft-notes.md") },
    restored: false,
  },
  {
    at: TimeMs.make(NOON - 7_200_000),
    path: "shop/notes/.cache/index.bin",
    restoration: { rebuildable: { reason: "rebuilt from the notes on the next read" } },
    restored: false,
  },
  {
    at: TimeMs.make(NOON - 9_000_000),
    path: "shop/checkout/old-plan.md",
    restoration: { interred: Locator.make("interred/2026-10-02/old-plan.md") },
    restored: false,
  },
  {
    at: TimeMs.make(NOON - 86_400_000),
    path: "shop/checkout/Plan.md",
    restoration: { tracked: Locator.make("1b2c3d4:shop/checkout/Plan.md") },
    restored: true,
  },
];

function answering(query: Query): Answer | undefined {
  if (typeof query === "object" && "archive_search" in query) {
    return { archive: { needle: query.archive_search.needle, hits: HITS } };
  }
  if (query === "discard_view") return { discards: { rows: DISCARDS } };
  return undefined;
}

import { For } from "solid-js";
import { say } from "../../core/lang";
import { storeValue } from "../../core/solid_store";
import { ui } from "../../ui";
import { Archive, Bin, RecordEntry } from "../record/history_views";
import Narrowing from "../record/narrowing.solid";
import { GalleryCase } from "./case";
import Stand from "./stand.solid";

const CITY = { kind: "live", city: "sprawling" } as const;
const WIDTH = [1440, 390] as const;
const RUNS = [
  { value: RUN, label: RUN.slice(0, 8) },
  { value: "8d0c", label: "8d0c41e2" },
];
const ignore = () => undefined;

export default function RecordedCases() {
  const lang = storeValue(ui().lang);
  return (
    <>
      <GalleryCase label="record · a ledger row opened to the record as it was written">
        <ul aria-label={say(lang(), "rec_timeline")}>
          <For each={ENTRIES}>
            {(row) => (
              <RecordEntry entry={row.entry} day={row.day} open={row.open} onToggle={ignore} />
            )}
          </For>
        </ul>
      </GalleryCase>
      <GalleryCase label="record · a narrowing choice holding one run">
        <Narrowing label={say(lang(), "log_runs")} options={RUNS} current={RUN} onPick={ignore} />
      </GalleryCase>
      <For each={WIDTH}>
        {(width) => (
          <GalleryCase
            label={`record · the archive with three hits at ${String(width)}`}
            width={width}
          >
            <Stand link={CITY} unread={[]} waiting={[]} answers={answering}>
              {() => <Archive />}
            </Stand>
          </GalleryCase>
        )}
      </For>
      <GalleryCase label="record · the recycle bin, one row for each way back and one already back">
        <Stand link={CITY} unread={[]} waiting={[]} answers={answering}>
          {() => <Bin />}
        </Stand>
      </GalleryCase>
    </>
  );
}
