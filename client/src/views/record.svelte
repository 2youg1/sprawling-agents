<script lang="ts" module>
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The one history in three readings: the timeline, where the ledger and
// the process log stand on one axis; the archive as the buildings filed
// it; and the recycle bin, where every row states its own way back.
// Nothing here is folded: the record is shown as the record.
//
// Which reading is shown is what the address bar says. The address
// grammar still has four record lenses (`core/route`): `ledger` and
// `log` both open the timeline, `log` with only the process log showing,
// so a link somebody kept to `#/record/log` reads what it always did.
// `parts/tabs` owns both halves of the tab-and-panel association
// (client-SPEC 7-8 item 6).

import type { Key } from "../core/lang";
import type { Lens } from "../core/route";
import type { Source } from "./record/timeline";

type Reading = "timeline" | "archive" | "bin";

const READINGS: readonly Reading[] = ["timeline", "archive", "bin"];

const READING_NAMES: Record<Reading, Key> = {
  timeline: "rec_timeline",
  archive: "rec_archive",
  bin: "rec_bin",
};

function readingOf(lens: Lens): Reading {
  switch (lens) {
    case "ledger":
    case "log":
      return "timeline";
    case "archive":
      return "archive";
    case "bin":
      return "bin";
  }
}

// The address a source of the timeline is kept under: the log alone has
// the old lens's address, and both other sources are the record's own.
function lensOf(source: Source): Lens {
  return source === "log" ? "log" : "ledger";
}
</script>

<script lang="ts">
  import type { Lens as Tab } from "./parts/tabs.svelte";
  import { say } from "../core/lang";
  import { ui } from "../ui";
  import Page from "./parts/page.svelte";
  import Tabs from "./parts/tabs.svelte";
  import Archive from "./record/archive.svelte";
  import Bin from "./record/bin.svelte";
  import Timeline from "./record/timeline.svelte";

  interface Props {
    readonly lens: Lens;
    // The shell draws this as a page; the gallery draws it as one region
    // among many, where it may not carry the page's heading.
    readonly rank?: "page" | "section" | undefined;
  }

  const { lens, rank = "page" }: Props = $props();

  const u = ui();
  const lang = u.lang;

  // The source the timeline reads. The address names the log; the
  // choice between both sources and the ledger alone is this page's,
  // and a move back from `#/record/log` drops a log-only choice.
  let picked = $state<Source>("every");
  const source = $derived<Source>(lens === "log" ? "log" : picked === "log" ? "every" : picked);

  const tabs = $derived(READINGS.map((each) => ({ id: each, label: say($lang, READING_NAMES[each]) })));

  function pick(id: string): void {
    const found = READINGS.find((each) => each === id);
    // A name this build does not read leaves the page where it is.
    if (found === undefined || found === readingOf(lens)) return;
    u.go({ kind: "record", lens: found === "timeline" ? lensOf(source) : found });
  }

  function choose(next: Source): void {
    picked = next;
    if (lensOf(next) !== lens) u.go({ kind: "record", lens: lensOf(next) });
  }
</script>

<Page title={say($lang, "nav_the_record")} {rank}>
  <Tabs label={say($lang, "rec_lenses")} lenses={tabs} current={readingOf(lens)} onPick={pick}>
    {#snippet panel(reading: Tab)}
      {#if reading.id === "timeline"}
        <Timeline {source} onSource={choose} />
      {:else if reading.id === "archive"}
        <Archive />
      {:else if reading.id === "bin"}
        <Bin />
      {/if}
    {/snippet}
  </Tabs>
</Page>
