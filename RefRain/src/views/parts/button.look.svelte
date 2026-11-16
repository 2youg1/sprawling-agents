<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  import type { Tone } from "./button";

  // The resting paint of each tone and the only hover it answers, with
  // the height each tone stands at on the control scale from
  // `theme/tokens-space.css`. The primary takes the large step because it
  // is "the one action a screen is for", which is that tier's own
  // definition; the other three take the ordinary step, the tier named
  // for a secondary button.
  //
  // The hover is written into the idle state rather than beside it: a
  // control that is waiting for an acknowledgement, or one a person may
  // not use, keeps its muted paint under the pointer instead of lighting
  // up as though the press would land.
  const WEAR: Record<Tone, string> = {
    primary: "h-control-lg bg-accent text-on-accent data-[state=idle]:hover:bg-accent-hover",
    secondary: "h-control bg-raised text-text data-[state=idle]:hover:bg-raised-hover",
    quiet: "h-control text-text-quiet data-[state=idle]:hover:bg-raised",
    destructive:
      "h-control bg-raised text-alert data-[state=idle]:hover:bg-alert data-[state=idle]:hover:text-on-accent",
  };

  // What every tone looks like once it is no longer idle. One rule for
  // both remaining states, because loading and refused are the same
  // answer to the hand: not now.
  const MUTED = "aria-disabled:bg-raised aria-disabled:text-text-disabled";

  // The shape, and the four properties that travel when it changes.
  // Background colour is among them so a hover arrives rather than
  // switching, which is what tells a hand the control heard it. The
  // resting state carries the leaving curve and the hovered and pressed
  // states the arriving one, because a transition takes the curve of the
  // state it enters (docs/frontend-method.md §4-43). The press gives by
  // the theme's one press ratio, and stands still
  // when the person or the machine asks for less motion.
  const SHAPE =
    "inline-flex items-center gap-snug rounded-control px-base text-label " +
    "transition-[background-color,color,opacity,transform] ease-leave " +
    "hover:ease-arrive active:ease-arrive active:scale-(--scale-press) " +
    "still:transition-none still:active:scale-100";
</script>

<script lang="ts">
  // How the one button is drawn, and nothing else: the label arrives in
  // the person's language and every property and handler arrives in the
  // wire bag (`./button.ts`), so another look draws the same control by
  // taking the same `ButtonLook`.
  import type { Snippet } from "svelte";

  import type { ButtonLook } from "./button";
  import Tip from "./tip.svelte";

  const look: ButtonLook = $props();

  // The checker types a `{#snippet}` name as a void call, which the
  // lint lane rejects inside a render tag. The name is taken again as
  // its `Snippet` type, and the template renders that.
  const control: Snippet<Parameters<typeof drawControl>> = drawControl;
</script>

{#snippet drawControl(hint: string | undefined)}
  <button {...look.wire(hint)} data-state={look.state} class="{SHAPE} {WEAR[look.tone]} {MUTED}">
    {#if look.state === "loading"}
      <span class="inline-block size-dot pulse rounded-pill bg-current"></span>
    {/if}
    {look.label}
  </button>
{/snippet}

{#if look.why === undefined}
  {@render control(undefined)}
{:else}
  <Tip text={look.why}>
    {#snippet children(hint: string)}
      {@render control(hint)}
    {/snippet}
  </Tip>
{/if}
