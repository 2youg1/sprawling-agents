<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  // How a copy key is drawn, and nothing else: its words arrive
  // translated and its press arrives wired (`./copy`, `CopyLook`).
  //
  // The named key stands at the ordinary control height, level with a
  // Button beside it in a command row; the bare key is the small square
  // of a header corner. Both widen their touch surface to the 44-point
  // floor with `::before` while the drawn control stays on the control
  // scale (docs/frontend-method.md §4-34). The receipt is a cut between
  // two marks, with no motion between them.
  import type { CopyForm } from "./copy";

  const SHAPE: Record<CopyForm, string> = {
    named: "h-control gap-tight px-snug text-label",
    bare: "size-control-sm justify-center",
  };
</script>

<script lang="ts">
  import Glyph from "./glyph.svelte";
  import Tip from "./tip.svelte";
  import type { CopyLook } from "./copy";

  const look: CopyLook = $props();
</script>

<Tip text={look.note}>
  {#snippet children(hint: string)}
    <button
      {...look.key}
      aria-describedby={hint}
      class={[
        "relative flex shrink-0 items-center rounded-control before:absolute before:-inset-snug before:content-['']",
        "transition-[background-color,color] ease-leave hover:bg-raised hover:ease-arrive still:transition-none",
        SHAPE[look.form],
        look.refused ? "text-alert" : "text-text-quiet hover:text-text",
      ]}
    >
      <Glyph name={look.face} size="sm" class="shrink-0" />
      {#if look.form === "named"}{look.name}{/if}
    </button>
  {/snippet}
</Tip>
<span class="sr-only" {...look.heard}>{look.said}</span>
