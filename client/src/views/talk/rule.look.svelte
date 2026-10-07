<!-- This Source Code Form is subject to the terms of the Mozilla Public
License, v. 2.0. If a copy of the MPL was not distributed with this
file, You can obtain one at https://mozilla.org/MPL/2.0/.
Copyright (c) 2026 2youg1 and the sprawling contributors -->

<!-- A boundary of the conversation: quiet words between two hairlines.
A link on it underlines under the pointer rather than taking a wash,
because it sits in a sentence; the fold is a press and takes the row's
wash. Stretches after the first are set off by a dot. The air above and
below is the seat's, like every gap between parts. -->
<script lang="ts">
  import Glyph from "../parts/glyph.svelte";
  import Fold from "./fold.look.svelte";
  import type { RuleLook } from "./rule";

  const look: RuleLook = $props();
</script>

<div
  {...look.wire}
  class="flex items-center gap-base text-note text-text-faint"
>
  <span class="h-px flex-1 bg-raised"></span>
  <span class="flex min-w-0 flex-wrap items-center justify-center gap-x-tight">
    {#if look.mark === "branch"}
      <Glyph name="branch" size="sm" />
    {/if}
    {#each look.parts as part, at (at)}
      {#if at > 0}
        <!-- wording-ok: a separator mark, not a word -->
        <span aria-hidden="true">·</span>
      {/if}
      {#if part.kind === "text"}
        <span>{part.text}</span>
      {:else if part.kind === "link"}
        <a href={part.href} class="underline-offset-2 hover:text-text-quiet hover:underline">{part.text}</a>
      {:else}
        <Fold {...part.fold} />
      {/if}
    {/each}
  </span>
  <span class="h-px flex-1 bg-raised"></span>
</div>
