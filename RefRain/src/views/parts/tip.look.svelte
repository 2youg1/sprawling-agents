<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  import type { Placing, Showing, TipSide } from "./tip";

  
  const AGAINST_WRAPPER: Record<TipSide, string> = {
    above: "absolute bottom-full left-1/2 -translate-x-1/2 mb-tight",
    right: "absolute left-full top-1/2 -translate-y-1/2 ml-base",
  };

  
  const AGAINST_ANCHOR: Record<TipSide, string> = {
    above:
      "supports-[anchor-name:--a]:fixed supports-[anchor-name:--a]:bottom-auto " +
      "supports-[anchor-name:--a]:left-auto supports-[anchor-name:--a]:translate-x-0 " +
      "supports-[anchor-name:--a]:[position-anchor:var(--tip-anchor)] " +
      "supports-[anchor-name:--a]:[position-area:block-start] " +
      "supports-[anchor-name:--a]:[position-try-fallbacks:flip-block]",
    right:
      "supports-[anchor-name:--a]:fixed supports-[anchor-name:--a]:top-auto " +
      "supports-[anchor-name:--a]:left-auto supports-[anchor-name:--a]:translate-y-0 " +
      "supports-[anchor-name:--a]:[position-anchor:var(--tip-anchor)] " +
      "supports-[anchor-name:--a]:[position-area:inline-end] " +
      "supports-[anchor-name:--a]:[position-try-fallbacks:flip-inline]",
  };

  function placed(placing: Placing, side: TipSide): readonly string[] {
    switch (placing) {
      case "by-engine":
        return [AGAINST_WRAPPER[side], AGAINST_ANCHOR[side]];
      case "against-wrapper":
        return [AGAINST_WRAPPER[side]];
    }
  }

  
  const PAINT =
    "pointer-events-none z-(--layer-popover) w-max max-w-measure rounded-card border border-edge-panel bg-raised " +
    "px-snug py-tight text-note text-text shadow-float " +
    "transition-[opacity,display] transition-discrete delay-300 duration-panel ease-leave " +
    "still:transition-none";

  
  const SHOWN: Record<Showing, string> = {
    wanted:
      "hidden opacity-0 group-hover/tip:block group-hover/tip:opacity-100 group-hover/tip:ease-arrive " +
      "group-focus-within/tip:block group-focus-within/tip:opacity-100 group-focus-within/tip:ease-arrive",
    dismissed: "hidden opacity-0",
    standing: "block opacity-100",
  };
</script>

<script lang="ts">
  import type { TipLook } from "./tip";

  const { holder, hint, id, anchor, text, side, placing, showing, children }: TipLook = $props();
</script>

<span
  class="group/tip relative inline-flex min-w-0 items-center [anchor-name:var(--tip-anchor)]"
  style:--tip-anchor={anchor}
  {...holder}
>
  {@render children(id)}
  <span {...hint} class={[PAINT, ...placed(placing, side), SHOWN[showing]]}>
    {text}
  </span>
</span>
