<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // The files a drop on the box did not take, each with the city's
  // reason beside its name, inline under the box the drop was aimed at
  // (`drop_zone.svelte.ts`).
  import { fill, say } from "../../core/lang";
  import { ui } from "../../ui";
  import type { Kept } from "./dropping";

  const { refused }: { readonly refused: readonly Kept[] } = $props();
  const { lang } = ui();
</script>

{#each refused as each (each.kind === "refused" ? each.name : "")}
  {#if each.kind === "refused"}
    <p class="text-note text-alert" role="alert">
      {fill(say($lang, "talk_drop_refused"), { name: each.name, why: each.said === "" ? say($lang, "talk_not_live") : each.said })}
    </p>
  {/if}
{/each}
