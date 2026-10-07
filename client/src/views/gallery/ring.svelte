<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  // The conversation's readings drawn from their looks with fixed
  // values, so every state stands still to be judged: the context ring
  // round the coin key, a sessions row's context bar, a reply's arrival
  // rhythm and the read-wear bar. A city in a fixture would draw only
  // the states it happened to be in; these are the ones a person meets.
  import type { Context } from "../talk/gauge";
  import type { Mark, Stretch, TrackWire } from "../talk/wear";

  const WINDOW = 200_000;

  // Used, first reminder, handoff: the three positions a ring is read at.
  const RINGS: readonly { readonly label: string; readonly context: Context }[] = [
    { label: "context ring · 41 % used, both reminders ahead", context: { used: 82_400, window: WINDOW, first: 30, second: 65 } },
    { label: "context ring · 88 % used, past the handoff", context: { used: 176_000, window: WINDOW, first: 30, second: 65 } },
    { label: "context ring · an older city that states no first reminder", context: { used: 30_000, window: WINDOW, first: null, second: 65 } },
  ];

  const BARS: readonly Context[] = [
    { used: 24_000, window: WINDOW, first: 30, second: 65 },
    { used: 150_000, window: WINDOW, first: 30, second: 65 },
    { used: 90_000, window: WINDOW, first: 30, second: null },
  ];

  // A steady stream, and one that stalled halfway and then came in a rush.
  const STEADY: readonly number[] = Array.from({ length: 20 }, (_, at) => 0.7 + (at % 3) * 0.1);
  const STALLED: readonly number[] = [0.4, 0.5, 0.6, 0.5, 0.4, 0.5, 0, 0, 0, 0, 0, 0, 0.1, 1, 0.9, 0.8, 0.6, 0.5, 0.4, 0.3];

  const MARKS: readonly Mark[] = [
    { top: 0.02, height: 0.04, phase: "model" },
    { top: 0.18, height: 0.06, phase: "tool" },
    { top: 0.31, height: 0.02, phase: "person" },
    { top: 0.47, height: 0.05, phase: "reply" },
    { top: 0.63, height: 0.08, phase: "tool" },
    { top: 0.995, height: 0.004, phase: "done" },
  ];
  const READ: readonly Stretch[] = [[0, 0.34], [0.58, 0.12]];
  const THUMB: Stretch = [0.6, 0.1];
  const IDLE: TrackWire = { onpointerdown: () => undefined, onpointermove: () => undefined };
</script>

<script lang="ts">
  import { say } from "../../core/lang";
  import { ui } from "../../ui";
  import Glyph from "../parts/glyph.svelte";
  import Ring from "../talk/gauge.look.svelte";
  import { barOf, ringOf } from "../talk/gauge";
  import Sparkline from "../talk/sparkline.svelte";
  import Wear from "../talk/wear.look.svelte";
  import Bar from "../world/context_bar.look.svelte";
  import Case from "./case.svelte";

  const { lang } = ui();
</script>

{#snippet key()}
  <span class="grid size-coin place-items-center text-text"><Glyph name="send" /></span>
{/snippet}

{#each RINGS as { label, context } (label)}
  <Case {label} width={120}>
    <div class="flex justify-center py-wide">
      <Ring ring={ringOf($lang, context)} children={key} />
    </div>
  </Case>
{/each}
<Case label="context ring · a model that states no window draws the key alone" width={120}>
  <div class="flex justify-center py-wide">
    <Ring ring={null} children={key} />
  </div>
</Case>

<Case label="context bar · early, past the handoff, and with no handoff stated" width={300}>
  <div class="flex flex-col gap-base py-snug">
    {#each BARS as context, at (at)}
      {@const bar = barOf($lang, context)}
      {#if bar !== null}
        <Bar {...bar} />
      {/if}
    {/each}
  </div>
</Case>

<Case label="arrival rhythm · a steady stream and one that stalled" width={300}>
  <div class="flex items-center gap-base text-note text-text-faint">
    <Sparkline rhythm={STEADY} />
    <Sparkline rhythm={STALLED} />
  </div>
</Case>

<Case label="read wear · every phase, two read stretches and the thumb" width={120}>
  <div class="relative h-output">
    <div class="absolute inset-y-0 right-0 w-snug">
      <Wear hint={say($lang, "talk_wear_hint")} track={IDLE} read={READ} thumb={THUMB} marks={MARKS} />
    </div>
  </div>
</Case>
