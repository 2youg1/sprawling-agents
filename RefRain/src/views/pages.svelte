<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  import type { View } from "../core/route";
  import { say } from "../core/lang";
  import { ui } from "../ui";
  import Building from "./building.svelte";
  import Raise from "./raise.svelte";
  import City from "./city.svelte";
  import Cost from "./cost.svelte";
  import Mcp from "./mcp.svelte";
  import Monitor from "./monitor.svelte";
  import RecordView from "./record.svelte";
  import Registry from "./registry.svelte";
  import Run from "./run.svelte";
  import Welcome from "./welcome.svelte";
  import { backLookOf } from "./pages/back";
  import Back from "./pages/back.look.svelte";

  interface Props {
    readonly view: View;
  }

  const { view }: Props = $props();
  const u = ui();
  const { lang } = u;
  const samples = u.conn.monitor.samples;
  const back = $derived(backLookOf(view, $lang));
</script>

<main
  id="main"
  class="col-[2/12] row-start-2 flex min-h-0 min-w-0 flex-col overflow-y-auto narrow:col-span-full"
  aria-label={say($lang, "region_main")}
>
  {#snippet backKey()}
    {#if back !== undefined}
      <div class="mb-snug">
        <Back {...back} />
      </div>
    {/if}
  {/snippet}
  {#if view.kind !== "welcome"}
    <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression -->
    {@render backKey()}
  {/if}
  {#if view.kind === "city"}
    <City />
  {:else if view.kind === "building"}
    <Building address={view.address} initialSection={view.section} />
  {:else if view.kind === "raise"}
    {#key view.address}<Raise address={view.address} />{/key}
  {:else if view.kind === "run"}
    {#key view.lens}<Run run={view.run} lens={view.lens} />{/key}
  {:else if view.kind === "mcp"}
    <Mcp />
  {:else if view.kind === "record"}
    <RecordView lens={view.lens} />
  {:else if view.kind === "cost"}
    <Cost />
  {:else if view.kind === "registry"}
    <Registry />
  {:else if view.kind === "welcome"}
    <Welcome step={view.step} back={backKey} />
  {:else if view.kind === "monitor"}
    <Monitor samples={$samples} watch={u.conn.monitor.watch} beat={u.conn.monitor.setBeat} />
  {:else if view.kind === "gallery"}
    {#await import("./gallery.svelte")}
      <div data-pending></div>
    {:then gallery}
      <gallery.default />
    {/await}
  {/if}
</main>
