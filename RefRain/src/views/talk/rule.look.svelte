<!-- This Source Code Form is subject to the terms of the Mozilla Public
License, v. 2.0. If a copy of the MPL was not distributed with this
file, You can obtain one at https://mozilla.org/MPL/2.0/.
Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  import Glyph from "../parts/glyph.svelte";
  import AsideLink from "./aside_link.look.svelte";
  import Fold from "./fold.look.svelte";
  import type { RuleLook } from "./rule";

  const look: RuleLook = $props();
</script>

<div {...look.wire} class="flex items-center gap-base text-note text-text-faint">
  <span class="h-px flex-1 bg-raised"></span>
  <span class="flex min-w-0 flex-wrap items-center justify-center gap-x-tight">
    {#if look.mark === "branch"}
      <Glyph name="branch" size="sm" />
    {/if}
    {#each look.parts as part, at (at)}
      {#if at > 0}
        <span aria-hidden="true">·</span>
      {/if}
      {#if part.kind === "text"}
        <span>{part.text}</span>
      {:else if part.kind === "link"}
        <AsideLink text={part.text} href={part.href} />
      {:else}
        <Fold {...part.fold} />
      {/if}
    {/each}
  </span>
  <span class="h-px flex-1 bg-raised"></span>
</div>
