<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  import type { Snippet } from "svelte";

  import { say } from "../../core/lang";
  import type { Utterance } from "./served";
  import { EARLIER_SEGMENT, ROUND } from "./served";

  const LINES: readonly Utterance[] = [...EARLIER_SEGMENT, ...ROUND];

  const SETTLED =
    "The mayor kept three things: the transcript of the first day, and the screenshot of the settings page under forced colours, filed where the registry lists them";
  const EDGE = " newest first";
</script>

<script lang="ts">
  import { ui } from "../../ui";
  import Case from "./case.svelte";

  const { lang } = ui();

  const drawUtterance: Snippet<Parameters<typeof utterance>> = utterance;
  const drawArriving: Snippet = arriving;
</script>

{#snippet utterance(line: Utterance)}
  {#if line.speaker === "person"}
    <div class="my-base flex flex-col items-end">
      <div
        class="max-w-[83%] rounded-panel bg-speech px-pane py-base text-body leading-read whitespace-pre-wrap"
      >
        {line.text}
      </div>
      <div class="mt-tight text-note text-text-faint">{say($lang, "talk_you")}</div>
    </div>
  {:else}
    <div class="my-base text-body">
      <div class="mb-tight text-note text-text-faint">{say($lang, "talk_resident")}</div>
      <div class="whitespace-pre-wrap leading-read">{line.text}</div>
    </div>
  {/if}
{/snippet}

{#snippet arriving()}
  <div class="my-base text-body">
    <div class="mb-tight text-note text-text-faint">{say($lang, "talk_resident")}</div>
    <div class="whitespace-pre-wrap leading-read">
      {SETTLED}<span class="text-text-faint">{EDGE}</span><span
        class="blink ml-tight inline-block size-[6px] bg-accent align-baseline"
      ></span>
    </div>
  </div>
{/snippet}

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
