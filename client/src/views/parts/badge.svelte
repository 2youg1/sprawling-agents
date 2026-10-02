<script lang="ts" module>
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// A small fact attached to something larger: how many are waiting, or
// where a thing stands. The word is always drawn, and the mark and the
// colour only repeat it - a state told by colour alone is a state that
// is not told to everybody.
//
// Two ways to be marked, and a badge takes exactly one. `status` takes
// both its drawing and its paint from the one status table in
// `glyph.ts`, so a chip here and a dot elsewhere tell the same story;
// `weight` and `dot` are the plain form for facts that have no state
// vocabulary, a count or a tier chosen by the caller. The union is what
// makes "a status painted at a caller's tier" unrepresentable: two
// authorities for one paint cannot meet in one badge.

import type { Status, Weight } from "./glyph";

export interface BadgeProps {
  // Already in the person's language, or a number the caller formatted.
  readonly text: string;
  readonly status?: Status;
  readonly weight?: undefined;
  readonly dot?: undefined;
  readonly seat?: Seat;
}

export interface BadgeTierProps {
  readonly text: string;
  readonly status?: undefined;
  // Nothing in particular, something going on, something wrong. The
  // three tiers of client/Spec.lean §4-32, painted below.
  readonly weight?: Weight;
  // A state reads better with a mark beside it; a count does not.
  readonly dot?: boolean;
  // Where the badge sits: in a line of text, or pinned to the corner of
  // a glyph, where it has to stay smaller than what it marks.
  readonly seat?: Seat;
}

export type Seat = "inline" | "corner";

// The resting paint of a tier and the fill of its round mark.
const PAINT: Record<Weight, string> = {
  quiet: "bg-raised text-text-quiet",
  live: "bg-raised text-accent",
  alert: "bg-raised text-alert",
};

const DOT: Record<Weight, string> = {
  quiet: "bg-mark",
  live: "bg-accent",
  alert: "bg-alert",
};

const SHAPE: Record<Seat, string> = {
  inline: "inline-flex items-center gap-tight rounded-pill px-snug py-tight text-note whitespace-nowrap",
  corner: "inline-flex items-center rounded-pill px-hair text-tally leading-none whitespace-nowrap",
};
</script>

<script lang="ts">
  import { statusLook } from "./glyph";
  import Glyph from "./glyph.svelte";

  const { text, status, weight, dot, seat }: BadgeProps | BadgeTierProps = $props();

  // The one reading of what this badge is marked with. Everything below
  // is four readers of these two lines and nothing else.
  const look = $derived(
    status === undefined ? undefined : statusLook(status),
  );
  const tier = $derived(look === undefined ? (weight ?? "quiet") : look.weight);
</script>

<span class={[SHAPE[seat ?? "inline"], PAINT[tier]]}>
  {#if look !== undefined}
    <Glyph name={look.glyph} size="sm" class="shrink-0" />
  {:else if dot === true}
    <span class={["inline-block size-dot rounded-pill", DOT[tier]]} aria-hidden="true"></span>
  {/if}
  {text}
</span>
