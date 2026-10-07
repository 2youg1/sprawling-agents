<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  // Which side of its control a hint stands on. Above, unless the
  // control stands at the window's foot with room only beside it - the
  // edge keys - where it stands to the right.
  export type TipSide = "above" | "right";

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

  // No stacking number: the hint is positioned and whatever it is drawn
  // over is not, which is already the order the two are painted in.
  //
  // The 300 ms delay is the whole point of the transition: it stops a
  // pointer crossing a row from lighting its hints one after another,
  // and client/Spec.lean §4-18 pins it. Only opacity moves, so the reveal
  // costs no layout, and `still` cuts it to nothing.
  const PAINT =
    "pointer-events-none w-max max-w-measure rounded-card border border-edge-panel bg-raised " +
    "px-snug py-tight text-note text-text shadow-float " +
    "transition-[opacity,display] transition-discrete delay-300 duration-panel ease-leave " +
    "still:transition-none";

  // Hidden costs nothing to draw and nothing to measure; shown is what
  // hover and focus both switch to.
  const WHEN_WANTED =
    "hidden opacity-0 group-hover/tip:block group-hover/tip:opacity-100 group-hover/tip:ease-arrive " +
    "group-focus-within/tip:block group-focus-within/tip:opacity-100 group-focus-within/tip:ease-arrive";

  // After Escape: hidden past both hover and focus, with no class that
  // could bring it back under them.
  const WHEN_DISMISSED = "hidden opacity-0";
</script>

<script lang="ts">
  // One hint, drawn beside the control it is about.
  //
  // This replaces `title`, which fails three readers: it waits about a
  // second for a pointer a keyboard never has, no key reaches it, and a
  // touch screen never draws it at all. The hint here appears on hover
  // and on focus anywhere inside the wrapper, so tabbing to the control
  // shows it.
  //
  // **The caller states the relation, because only the caller knows
  // whether the control has a name already.** The child is handed the
  // hint's id and writes `aria-describedby` when it is named by its own
  // text, or `aria-labelledby` when these words are the only name it
  // has. Writing neither leaves a hint a screen reader never reads.
  // A part split into a seat and a look (client D95) passes `id`
  // instead: the seat chose the id and already wrote the relation into
  // the control's wire bag, so the look hands the same id here and
  // writes no ARIA of its own.
  //
  // **Placement has two branches, because engines disagree.** Where
  // anchor positioning is implemented the hint is `fixed` against the
  // anchor and leaves every box that clips behind it. Where it is not -
  // Safari and Firefox today - the hint is placed against the wrapper,
  // which is why the wrapper is `relative`. Shipping only the first
  // branch fails silently: the hint keeps a static position and can land
  // outside the window.
  //
  // **Escape dismisses the hint and nothing else** (client/Spec.lean §7-3,
  // WCAG 1.4.13). It stays away while the pointer or the focus stays,
  // and the next re-engagement summons it back. The dismissal listens
  // at the window because a hint raised by hover has no focused element
  // to hear the key; the two re-engagement listeners ride an attachment
  // on the wrapper for the same reason the rest of this does not: a
  // wrapper is not a control and must not grow an interactive role just
  // to hold two listeners.
  import type { Attachment } from "svelte/attachments";
  import { on } from "svelte/events";
  import type { Snippet } from "svelte";

  interface Props {
    // Already in the person's language.
    readonly text: string;
    // The control the hint is about, handed the hint's id: as
    // `aria-describedby` when the control is named by its own text, as
    // `aria-labelledby` when these words are that name.
    readonly children: Snippet<[string]>;
    // The hint's id, when the caller decided it; otherwise one is made.
    readonly id?: string;
    readonly side?: TipSide;
    // Whether holding the accelerator alone draws this hint with the
    // others (docs/frontend-method.md §7E): the names of the edge keys are, a hint
    // inside a form is not.
    readonly exposable?: boolean;
  }

  const { text, children, id, side = "above", exposable = false }: Props = $props();

  const own = $props.id();
  const hint = $derived(id ?? own);

  let dismissed = $state(false);

  // One anchor name per instance, carried to the hint by inheritance, so
  // two hints on one row anchor to their own control rather than both to
  // whichever came last in the document.
  const anchor = $derived(`--tip-${hint}`);

  const reengage: Attachment = (node) => {
    const enter = on(node, "mouseenter", () => {
      dismissed = false;
    });
    const focus = on(node, "focusin", () => {
      dismissed = false;
    });
    return () => {
      enter();
      focus();
    };
  };
</script>

<svelte:window
  onkeydown={(event) => {
    if (event.key === "Escape") dismissed = true;
  }}
/>

<span
  class="group/tip relative inline-flex min-w-0 items-center [anchor-name:var(--tip-anchor)]"
  style:--tip-anchor={anchor}
  {@attach reengage}
>
  {@render children(hint)}
  <span
    id={hint}
    role="tooltip"
    class={[PAINT, AGAINST_WRAPPER[side], AGAINST_ANCHOR[side], dismissed ? WHEN_DISMISSED : WHEN_WANTED]}
    data-exposable={exposable && !dismissed ? "" : undefined}
  >
    {text}
  </span>
</span>
