<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  // A small square on the control scale with the small icon in it, so a
  // key that sits inside a line or a row does not stand taller than the
  // text beside it. Under a coarse pointer the square keeps its drawn
  // size and only the area that takes the touch grows, by a `::before`
  // centred on the key, to the theme's touch square (§4-34); a fine
  // pointer already lands on 28 px. The ink brightens under the pointer
  // only while the key can be used, arriving on the arriving curve and
  // leaving on the leaving one (§4-43).
  const KEY =
    "relative inline-flex size-control-sm shrink-0 items-center justify-center rounded-control " +
    "text-text-faint transition-colors ease-leave aria-[disabled=false]:hover:text-text-quiet " +
    "hover:ease-arrive still:transition-none";

  const TOUCH =
    "pointer-coarse:before:absolute pointer-coarse:before:top-1/2 pointer-coarse:before:left-1/2 " +
    "pointer-coarse:before:size-touch pointer-coarse:before:-translate-1/2 pointer-coarse:before:content-['']";
</script>

<script lang="ts">
  // How the one icon-only key is drawn, and nothing else: the name, the
  // ARIA properties and the press guard arrive in the bag
  // (`./icon_button.ts`), so another look draws the same key by taking
  // the same `IconButtonLook`.
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
