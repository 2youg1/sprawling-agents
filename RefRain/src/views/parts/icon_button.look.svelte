<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  const KEY =
    "relative inline-flex size-control-sm shrink-0 items-center justify-center rounded-control " +
    "text-text-faint transition-[color,scale] ease-leave " +
    "aria-[disabled=false]:hover:text-text-quiet hover:ease-arrive " +
    "aria-[disabled=false]:active:scale-(--scale-press) aria-[disabled=false]:active:ease-arrive " +
    "still:transition-none still:aria-[disabled=false]:active:scale-100";

  const TOUCH =
    "pointer-coarse:before:absolute pointer-coarse:before:top-1/2 pointer-coarse:before:left-1/2 " +
    "pointer-coarse:before:size-touch pointer-coarse:before:-translate-1/2 pointer-coarse:before:content-['']";
</script>

<script lang="ts">
  import Glyph from "./glyph.svelte";
  import type { IconButtonLook } from "./icon_button";
  import Tip from "./tip.svelte";

  const look: IconButtonLook = $props();
</script>

<Tip text={look.hint}>
  {#snippet children(hint: string)}
    <button {...look.wire(hint)} class="{KEY} {TOUCH}">
      <Glyph name={look.glyph} size="sm" />
    </button>
  {/snippet}
</Tip>
