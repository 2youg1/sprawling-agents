<script lang="ts" module>
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// How far along something is, as a bar and as the two numbers the bar
// was drawn from. The numbers are there because a bar alone cannot be
// read out, compared, or believed.
//
// A total of zero means the end is not known yet: the bar then says it
// is busy instead of claiming a fraction it does not have. The three
// ARIA values follow that one question exactly - the lower bound is
// always there, the upper bound and the current value appear only when
// there is an end to be at, and the bar is busy in every other case.

export interface ProgressProps {
  // The accessible name of the bar, already in the person's language.
  readonly label: string;
  readonly done: number;
  // Zero or less means the end is unknown.
  readonly total: number;
}
</script>

<script lang="ts">
  const { label, done, total }: ProgressProps = $props();

  const known = $derived(total > 0);
  const share = $derived(
    known ? Math.min(Math.max(done / total, 0), 1) : 0,
  );
</script>

<div class="flex w-full min-w-0 items-center gap-base">
  <div
    class="h-snug min-w-0 flex-1 overflow-hidden rounded-pill bg-raised"
    role="progressbar"
    aria-label={label}
    aria-valuemin={0}
    aria-valuemax={known ? total : undefined}
    aria-valuenow={known ? done : undefined}
    aria-busy={known ? "false" : "true"}
  >
    {#if known}
      <div
        class="h-full rounded-pill bg-progress-done"
        style:width="{Math.round(share * 100)}%"
      ></div>
    {:else}
      <div class="shimmer h-full w-1/3 rounded-pill"></div>
    {/if}
  </div>
  {#if known}
    <span class="shrink-0 font-mono text-note text-text-quiet">{done} / {total}</span>
  {/if}
</div>
