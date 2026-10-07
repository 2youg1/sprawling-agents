<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<!-- The sandbox of the room's building as a read-only fact in the shape
of the settings row's chips. Nothing is drawn while no sandbox restricts
a run there (`sandbox.ts`), so the row never says "none". -->
<script lang="ts">
  import { readable } from "svelte/store";
  import { ui } from "../../ui";
  import type { Address } from "../../wire";
  import Fact from "./fact.look.svelte";
  import { restriction, sandboxQuery, sandboxSaid } from "./sandbox";

  interface Props {
    readonly room: Address | null;
  }

  const { room }: Props = $props();

  const u = ui();
  const { lang } = u;
  const building = $derived(room === null ? readable(undefined) : u.conn.asking.ask(sandboxQuery(room)));
  const limits = $derived(restriction($building));
</script>

{#if limits !== null}
  <Fact glyph="sandbox"><span class="truncate">{sandboxSaid(limits, $lang)}</span></Fact>
{/if}
