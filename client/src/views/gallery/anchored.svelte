<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  // The thread in the two states its anchoring decides between (ux B1),
  // each drawn as the viewport shows it: one conversation in one
  // scroller, once at the foot where the growing edge is followed, and
  // once left mid-history where the person's place is held while the
  // same words arrive below the fold. Both specimens carry identical
  // content, so the only difference a person judges is which end of it
  // the box shows.
  //
  // The reply still arriving is stated as two fixture values - what has
  // settled and what is still faint at the growing edge - rather than
  // one string cut at a number: the live thread owns that number, and
  // the `saying` fixtures beside these are where its size is judged.

  import type { Snippet } from "svelte";

  import { say } from "../../core/lang";
  import type { Utterance } from "./served";
  import { EARLIER_SEGMENT, ROUND } from "./served";

  // The one thread both specimens draw: the segment the session
  // divider folds away, then this round, then the reply arriving.
  const LINES: readonly Utterance[] = [...EARLIER_SEGMENT, ...ROUND];

  const SETTLED =
    "The mayor kept three things: the transcript of the first day, and the screenshot of the settings page under forced colours, filed where the registry lists them";
  const EDGE = " newest first";
</script>

<script lang="ts">
  import { ui } from "../../ui";
  import Case from "./case.svelte";

  const { lang } = ui();

  // The checker types a `{#snippet}` name as a void call, which the
  // lint lane rejects inside a render tag. Each name is taken again as
  // its `Snippet` type, and the template renders that.
  const drawUtterance: Snippet<Parameters<typeof utterance>> = utterance;
  const drawArriving: Snippet = arriving;
</script>

{#snippet utterance(line: Utterance)}
  {#if line.speaker === "person"}
    <div class="my-base flex flex-col items-end">
      <div class="max-w-[83%] rounded-panel bg-speech px-pane py-base text-body leading-relaxed whitespace-pre-wrap">
        {line.text}
      </div>
      <div class="mt-tight text-note text-text-disabled">{say($lang, "talk_you")}</div>
    </div>
  {:else}
    <div class="my-base text-body">
      <div class="mb-tight text-note text-text-disabled">{say($lang, "talk_resident")}</div>
      <div class="whitespace-pre-wrap leading-relaxed">{line.text}</div>
    </div>
  {/if}
{/snippet}

<!-- The reply as the thread draws it mid-turn: the settled words, the
     last characters faint so text emerges instead of appearing, and
     the cursor the `blink` animation carries. -->
{#snippet arriving()}
  <div class="my-base text-body">
    <div class="mb-tight text-note text-text-disabled">{say($lang, "talk_resident")}</div>
    <div class="whitespace-pre-wrap leading-relaxed">
      {SETTLED}<span class="text-text-faint">{EDGE}</span><span
        class="blink ml-tight inline-block size-[6px] bg-accent align-baseline"
      ></span>
    </div>
  </div>
{/snippet}

<!-- At the foot: `justify-end` packs the thread to the foot of its
     scroller exactly as the live column does, so the growing edge is
     what the viewport holds and the append follows it. -->
<Case label="thread · at the foot, the growing edge followed">
  <div class="h-output overflow-y-auto">
    <div class="flex min-h-full w-full flex-col justify-end px-pane pb-wide">
      {#each LINES as line (line.text)}
        {@render drawUtterance(line)}
      {/each}
      {@render drawArriving()}
    </div>
  </div>
</Case>

<!-- Reading back: the same thread with the foot free, so the viewport
     sits in the history and the words arriving below it change nothing
     on screen. -->
<Case label="thread · reading back, the place held while words arrive">
  <div class="h-output overflow-y-auto">
    <div class="flex min-h-full w-full flex-col px-pane pb-wide">
      {#each LINES as line (line.text)}
        {@render drawUtterance(line)}
      {/each}
      {@render drawArriving()}
    </div>
  </div>
</Case>
