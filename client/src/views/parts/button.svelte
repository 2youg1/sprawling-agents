<script lang="ts" module>
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The one button in the client. Four tones, because a page has four
// kinds of thing to ask for; a loading posture, because a command frame
// is answered later than the hand that sent it; and a refusal that
// carries its reason, because a control a person cannot use owes them
// the sentence saying why.
//
// The words are the caller's: this file holds no prose.

// primary is the one thing the screen is for, secondary the ones beside
// it, quiet the ones that must not compete, destructive the one that
// cannot be taken back.
export type Tone = "primary" | "secondary" | "quiet" | "destructive";

// What the control is doing, as one word the stylesheet reads off the
// element. Every posture the button can take is a value here, so a
// reader asks the DOM what state it is in rather than deducing it from
// which classes happen to be on it.
type State = "idle" | "loading" | "stopped";

export interface ButtonProps {
  // Already in the person's language: a `say` key resolved by the page.
  readonly label: string;
  readonly tone?: Tone;
  readonly type?: "button" | "submit";
  // The command went out and its acknowledgement has not come back.
  readonly loading?: boolean;
  // Present means the control cannot be used, and says why. It stays
  // focusable and keeps `aria-disabled`, because a `disabled` element is
  // skipped by the keyboard and its reason is never read out. The reason
  // is drawn beside the button rather than left in a `title`, which a
  // keyboard never reaches and a touch screen never shows.
  readonly why?: string;
  readonly onPress?: () => void;
}

// The resting paint of each tone and the only hover it answers, with the
// height each tone stands at on the control scale from `theme.css`. The
// primary takes the large step because it is "the one action a screen is
// for", which is that tier's own definition; the other three take the
// ordinary step, the tier named for a secondary button.
//
// The hover is written into the idle state rather than beside it: a
// control that is waiting for an acknowledgement, or one a person may
// not use, keeps its muted paint under the pointer instead of lighting
// up as though the press would land.
const WEAR: Record<Tone, string> = {
  primary:
    "h-control-lg bg-accent text-on-accent data-[state=idle]:hover:bg-accent-hover",
  secondary:
    "h-control bg-raised text-text data-[state=idle]:hover:bg-raised-hover",
  quiet: "h-control text-text-quiet data-[state=idle]:hover:bg-raised",
  destructive:
    "h-control bg-raised text-alert data-[state=idle]:hover:bg-alert data-[state=idle]:hover:text-on-accent",
};

// What every tone looks like once it is no longer idle. One rule for
// both remaining states, because loading and refused are the same
// answer to the hand: not now.
const MUTED =
  "aria-disabled:bg-raised aria-disabled:text-text-disabled";

// The shape, and the four properties that travel when it changes.
// Background colour is among them so a hover arrives rather than
// switching, which is what tells a hand the control heard it. The press
// and the pulse move transform and opacity only, and both stand still
// for a machine that asks for less motion.
const SHAPE =
  "inline-flex items-center gap-snug rounded-control px-base text-label " +
  "transition-[background-color,color,opacity,transform] duration-100 ease-standard " +
  "active:scale-[0.98] motion-reduce:transition-none motion-reduce:active:scale-100";
</script>

<script lang="ts">
  import type { Snippet } from "svelte";

  import Tip from "./tip.svelte";

  const {
    label,
    tone = "secondary",
    type = "button",
    loading = false,
    why,
    onPress,
  }: ButtonProps = $props();

  // The one reading of what the control is doing. The attribute, the two
  // ARIA properties and the click guard are four readers of this line,
  // and nothing else decides the question.
  const state = $derived.by((): State => {
    if (why !== undefined) return "stopped";
    return loading ? "loading" : "idle";
  });

  // The checker types a `{#snippet}` name as a void call, which the
  // lint lane rejects inside a render tag. The name is taken again as
  // its `Snippet` type, and the template renders that.
  const control: Snippet<Parameters<typeof drawControl>> = drawControl;
</script>

<!-- One definition of the control, drawn bare or inside its reason.
Enter and Space are both delivered as a click by the platform, so the
single guard below is what stands between every input and `onPress`. -->
{#snippet drawControl(hint: string | undefined)}
  <button
    {type}
    data-state={state}
    class="{SHAPE} {WEAR[tone]} {MUTED}"
    aria-disabled={state !== "idle"}
    aria-busy={state === "loading"}
    aria-describedby={hint}
    onclick={() => {
      if (state !== "idle") return;
      onPress?.();
    }}
  >
    {#if state === "loading"}
      <span class="inline-block size-dot animate-pulse rounded-pill bg-current"></span>
    {/if}
    {label}
  </button>
{/snippet}

{#if why === undefined}
  {@render control(undefined)}
{:else}
  <!-- The reason is a `Tip`, and this caller writes the relation: the
  button is named by its own text, so the hint is `aria-describedby`. -->
  <Tip text={why}>
    {#snippet children(hint: string)}
      {@render control(hint)}
    {/snippet}
  </Tip>
{/if}
