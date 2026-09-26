<script lang="ts" module>
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The one history in four lenses: the ledger as it was written, the
// archive as the buildings filed it, the recycle bin where every row
// states its own way back, and the process log beside them. Nothing
// here is folded: the record is shown as the record.
//
// Which lens is shown is what the address bar says, for all four of
// them: `core/route` spells the log as `#/record/log`, so a person can
// send somebody a link to what their machine was writing. `parts/tabs`
// owns both halves of the tab-and-panel association (client-SPEC 7-8
// item 6), so each reading mounts while its lens is current and
// unmounts when the lens leaves.

import type { Key } from "../core/lang";
import type { Lens } from "../core/route";

const LENS_NAMES: Record<Lens, Key> = {
  ledger: "rec_ledger",
  archive: "rec_archive",
  bin: "rec_bin",
  log: "rec_log",
};
</script>

<script lang="ts">
  import { LENSES } from "../core/route";
  import type { Lens as Reading } from "./parts/tabs.svelte";
  import { say } from "../core/lang";
  import { ui } from "../ui";
  import Page from "./parts/page.svelte";
  import Tabs from "./parts/tabs.svelte";
  import Archive from "./record/archive.svelte";
  import Bin from "./record/bin.svelte";
  import Ledger from "./record/ledger.svelte";
  import Log from "./record/log.svelte";

  interface Props {
    readonly lens: Lens;
  }

  const { lens }: Props = $props();

  const u = ui();
  const lang = u.lang;

  const lenses = $derived(
    LENSES.map((each) => ({ id: each, label: say($lang, LENS_NAMES[each]) })),
  );

  function pick(id: string): void {
    const found = LENSES.find((each) => each === id);
    // A name this build does not read leaves the page where it is,
    // rather than moving somebody somewhere they did not ask for.
    if (found === undefined) {
      return;
    }
    u.go({ kind: "record", lens: found });
  }
</script>

<Page title={say($lang, "nav_the_record")}>
  <Tabs label={say($lang, "rec_lenses")} {lenses} current={lens} onPick={pick}>
    {#snippet panel(reading: Reading)}
      {#if reading.id === "ledger"}
        <Ledger />
      {:else if reading.id === "archive"}
        <Archive />
      {:else if reading.id === "bin"}
        <Bin />
      {:else if reading.id === "log"}
        <Log />
      {/if}
    {/snippet}
  </Tabs>
</Page>
