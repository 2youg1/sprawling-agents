<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  // Every page but the conversation, in the columns right of the edge
  // keys (client/Spec.lean §4-33), until each is redesigned for the shell. The
  // conversation page lays itself out by the tier and is not here
  // (`workspace.svelte`).
  import type { View } from "../core/route";
  import { say } from "../core/lang";
  import { ui } from "../ui";
  import Building from "./building.svelte";
  import City from "./city.svelte";
  import Cost from "./cost.svelte";
  import Mcp from "./mcp.svelte";
  import Monitor from "./monitor.svelte";
  import RecordView from "./record.svelte";
  import Registry from "./registry.svelte";
  import Run from "./run.svelte";
  import Setup from "./setup.svelte";
  import Welcome from "./welcome.svelte";

  interface Props {
    readonly view: View;
  }

  const { view }: Props = $props();
  const u = ui();
  const { lang } = u;
  const samples = u.conn.monitor.samples;
</script>

<main
  id="main"
  class="col-[2/-1] row-start-2 flex min-h-0 min-w-0 flex-col overflow-y-auto narrow:col-span-full"
  aria-label={say($lang, "region_main")}
>
  {#if view.kind === "city"}
    <City />
  {:else if view.kind === "building"}
    <Building address={view.address} />
  {:else if view.kind === "run"}
    <Run run={view.run} />
  {:else if view.kind === "setup"}
    <Setup />
  {:else if view.kind === "mcp"}
    <Mcp />
  {:else if view.kind === "record"}
    <RecordView lens={view.lens} />
  {:else if view.kind === "cost"}
    <Cost />
  {:else if view.kind === "registry"}
    <Registry />
  {:else if view.kind === "welcome"}
    <Welcome />
  {:else if view.kind === "monitor"}
    <Monitor samples={$samples} watch={u.conn.monitor.watch} />
  {:else if view.kind === "gallery"}
    <!-- The storybook and its fixture tables are a chunk of their own,
         fetched only when a person opens `#/gallery`, so the page every
         other route loads does not carry them. -->
    {#await import("./gallery.svelte")}
      <!-- Pending until the chunk lands; the render gate waits for this
           mark to go before it measures the page. -->
      <div data-pending></div>
    {:then gallery}
      <gallery.default />
    {/await}
  {/if}
</main>
