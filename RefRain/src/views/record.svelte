<script lang="ts" module>
  
  
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
    
    
    readonly rank?: "page" | "section" | undefined;
  }

  const { lens, rank = "page" }: Props = $props();

  const u = ui();
  const lang = u.lang;

  
  let picked = $state<Source>("every");
  const source = $derived<Source>(lens === "log" ? "log" : picked === "log" ? "every" : picked);

  const tabs = $derived(
    READINGS.map((each) => ({ id: each, label: say($lang, READING_NAMES[each]) })),
  );

  function pick(id: string): void {
    const found = READINGS.find((each) => each === id);
    
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
