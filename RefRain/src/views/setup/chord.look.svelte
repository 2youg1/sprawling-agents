<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  import { Kbd } from "../parts/kbd.svelte";
  import Tip from "../parts/tip.svelte";
  import type { ChordLook } from "./chord";

  const { wire, action, spelled, word, tip }: ChordLook = $props();
</script>

<!-- `Tip` hands out the id of the line it draws, and the button names it
as its description; that one attribute is written here until the tip
takes the id from its caller. -->
<Tip text={tip}>
  {#snippet children(hint: string)}
    <button
      {...wire}
      aria-describedby={hint}
      class="h-control-sm rounded-control border border-edge px-snug transition-colors duration-short ease-leave hover:bg-raised hover:ease-arrive aria-pressed:bg-raised-hover"
    >
      {#if word !== undefined}
        <span class="font-mono text-note text-text-faint">{word}</span>
      {:else}
        <!-- The chord is redrawn whenever it changes: the marks read
             `core/keys` outside the reactive surface, so the key here is
             what brings them back. -->
        {#key spelled}
          <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression, @typescript-eslint/no-unsafe-call (a snippet call is the render itself, typed `void`; a snippet exported from another component resolves for `svelte-check` but not for the type-aware lint lane) -->
          {@render Kbd({ action })}
        {/key}
      {/if}
    </button>
  {/snippet}
</Tip>
