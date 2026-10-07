<!-- This Source Code Form is subject to the terms of the Mozilla Public
License, v. 2.0. If a copy of the MPL was not distributed with this
file, You can obtain one at https://mozilla.org/MPL/2.0/.
Copyright (c) 2026 2youg1 and the sprawling contributors -->

<!-- How the branch action is drawn. Like every button (D55) it carries a
mark the eye knows - the branch glyph -, its name in words beside the
mark, and a hint on hover and focus that says what a branch keeps.

It stands at the end of its entry's own line and never over the words,
because a button laid over text hides the text it would branch from. It
keeps its place while hidden, so revealing it moves nothing. The nearest
`group` around it is the entry it reveals with: it fades in with the
arriving curve and out with the leaving one, and `data-fork` lets a
fixture hold it revealed. -->
<script lang="ts">
  import Glyph from "../parts/glyph.svelte";
  import Tip from "../parts/tip.svelte";
  import type { ForkButtonLook } from "./fork_button";

  const look: ForkButtonLook = $props();
</script>

<div
  data-fork
  class="shrink-0 opacity-0 transition-opacity ease-leave group-focus-within:opacity-100 group-focus-within:ease-arrive group-hover:opacity-100 group-hover:ease-arrive focus-within:opacity-100 focus-within:ease-arrive"
>
  <Tip text={look.hint}>
    {#snippet children(hint: string)}
      <button
        {...look.wire}
        aria-describedby={hint}
        class="flex h-control-sm items-center gap-tight rounded-control px-tight text-note whitespace-nowrap text-text-faint hover:wash hover:text-text-quiet"
      >
        <Glyph name="branch" size="sm" />
        {look.label}
      </button>
    {/snippet}
  </Tip>
</div>
