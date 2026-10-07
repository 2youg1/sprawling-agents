<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  // The record's rows and its two other readings: a timeline row opened
  // to the record as it was written, beside a closed one and a log line
  // the page holds without a time; a narrowing choice holding one run;
  // the archive with what it found; and the recycle bin, one row for each
  // way back, one of them already back.

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
  const STARTED: EventRecord = { ...RECORD, seq: Seq.make(40), kind: "run_started", t: TimeMs.make(NOON - 2_412), addr: null, data: { task: "tidy the release notes" } };
  const UNTIMED: LogLine = { seq: Seq.make(44), t: null, level: "effect", module: "memory", line: "commit 9c41e07 on shop/notes", run: RUN };

  const ENTRIES: readonly { readonly entry: Entry; readonly day: string | null; readonly open: boolean }[] = [
    { entry: { kind: "log", key: "l44", seq: UNTIMED.seq, t: null, line: UNTIMED }, day: null, open: false },
    { entry: { kind: "record", key: "r44", seq: RECORD.seq, t: RECORD.t, record: RECORD }, day: "2026-10-02", open: true },
    { entry: { kind: "record", key: "r40", seq: STARTED.seq, t: STARTED.t, record: STARTED }, day: null, open: false },
  ];

  const HITS: readonly ArchiveHit[] = [
    { day: 20_363, kind: "memo", building: Address.make("shop/notes"), subject: "why the changelog keeps one entry per card" },
    { day: 20_361, kind: "plan", building: Address.make("shop/checkout"), subject: "move the receipt printer behind the payment step" },
    { day: 20_354, kind: "spec", building: Address.make("hall/registry"), subject: "every resident names the building it was placed in" },
  ];

  const DISCARDS: readonly DiscardLine[] = [
    { at: TimeMs.make(NOON - 3_600_000), path: "shop/notes/draft-notes.md", restoration: { tracked: Locator.make("9c41e07:shop/notes/draft-notes.md") }, restored: false },
    { at: TimeMs.make(NOON - 7_200_000), path: "shop/notes/.cache/index.bin", restoration: { rebuildable: { reason: "rebuilt from the notes on the next read" } }, restored: false },
    { at: TimeMs.make(NOON - 9_000_000), path: "shop/checkout/old-plan.md", restoration: { interred: Locator.make("interred/2026-10-02/old-plan.md") }, restored: false },
    { at: TimeMs.make(NOON - 86_400_000), path: "shop/checkout/Plan.md", restoration: { tracked: Locator.make("1b2c3d4:shop/checkout/Plan.md") }, restored: true },
  ];

  function answering(query: Query): Answer | undefined {
    if (typeof query === "object" && "archive_search" in query) {
      return { archive: { needle: query.archive_search.needle, hits: HITS } };
    }
    if (query === "discard_view") return { discards: { rows: DISCARDS } };
    return undefined;
  }
</script>

<script lang="ts">
  import { say } from "../../core/lang";
  import { ui } from "../../ui";
  import Archive from "../record/archive.svelte";
  import Bin from "../record/bin.svelte";
  import Row from "../record/entry.svelte";
  import Narrowing from "../record/narrowing.svelte";
  import Case from "./case.svelte";
  import Stand from "./stand.svelte";

  const lang = ui().lang;
</script>

<Case label="record · a ledger row opened to the record as it was written">
  <ul aria-label={say($lang, "rec_timeline")}>
    {#each ENTRIES as shown (shown.entry.key)}
      <Row entry={shown.entry} day={shown.day} open={shown.open} onToggle={() => undefined} />
    {/each}
  </ul>
</Case>

<Case label="record · a narrowing choice holding one run">
  <Narrowing
    label={say($lang, "log_runs")}
    options={[{ value: RUN, label: RUN.slice(0, 8) }, { value: "8d0c", label: "8d0c41e2" }]}
    current={RUN}
    onPick={() => undefined}
  />
</Case>

{#each [1440, 390] as width (width)}
  <Case label={`record · the archive with three hits at ${String(width)}`} {width}>
    <Stand link={{ kind: "live", city: "sprawling" }} unread={[]} waiting={[]} answers={answering}>
      <Archive />
    </Stand>
  </Case>
{/each}

<Case label="record · the recycle bin, one row for each way back and one already back">
  <Stand link={{ kind: "live", city: "sprawling" }} unread={[]} waiting={[]} answers={answering}>
    <Bin />
  </Stand>
</Case>
