<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  import type { Placing, Showing, TipSide } from "./tip";

  // Against the wrapper, for an engine without anchor positioning.
  const AGAINST_WRAPPER: Record<TipSide, string> = {
    above: "absolute bottom-full left-1/2 -translate-x-1/2 mb-tight",
    right: "absolute left-full top-1/2 -translate-y-1/2 ml-base",
  };

  // Against the anchor, for an engine with it. `flip-block` drops the
  // hint below the control when there is no room above.
  //
  // `--tip-anchor` is spelled out rather than built from a constant
  // because Tailwind reads class names out of this file as text: a name
  // assembled at run time produces no CSS at all.
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

  // No stacking number: the hint is positioned and whatever it is drawn
  // over is not, which is already the order the two are painted in.
  //
  // The 300 ms delay is the whole point of the transition: it stops a
  // pointer crossing a row from lighting its hints one after another,
  // and client/Spec.lean §4-18 pins it. It is not `HOLD_MS` in
  // `core/press.ts`, though both read 300: that one tells a key held
  // from a key pressed and also decides when the layers key peeks at
  // the blend tier, and hold-to-reveal (theme/expose.css) zeroes this
  // delay because the hold has already waited. Tuning how long a pointer
  // must rest on a dense row must not move when Accel-\ peeks.
  //
  // Only opacity moves, so the reveal costs no layout, and `still` cuts
  // it to nothing. The resting state leaves and the shown state arrives
  // (docs/frontend-method.md §4-43).
  const PAINT =
    "pointer-events-none w-max max-w-measure rounded-card border border-edge-panel bg-raised " +
    "px-snug py-tight text-note text-text shadow-float " +
    "transition-[opacity,display] transition-discrete delay-300 duration-panel ease-leave " +
    "still:transition-none";

  // Hidden costs nothing to draw and nothing to measure; shown is what
  // hover and focus both switch to. After Escape: hidden past both hover
  // and focus, with no class that could bring it back under them.
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
