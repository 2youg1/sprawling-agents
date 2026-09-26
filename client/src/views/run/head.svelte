<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // The run page's summary bar: how the run stands, how long it took,
  // what it read and wrote in tokens, what it cost, and where it lives,
  // as one line of labelled facts that wraps on a narrow page. Every
  // figure is summed from the rounds the page already asked for, so the
  // bar and the lenses under it cannot disagree.

  import type { Doing } from "../../core/doing";
  import { POSTURE_WORD } from "../../core/doing";
  import { fill, say } from "../../core/lang";
  import { buildingOf, roomOf, toFragment } from "../../core/route";
  import { clock, count, lasted, usd } from "../../core/time";
  import { ui } from "../../ui";
  import type { Address, Closing, Turn } from "../../wire";
  import { figuresOf, modelsOf } from "./lanes";

  interface Props {
    readonly turns: readonly Turn[];
    readonly doing: Doing | undefined;
    readonly closing: Closing | null;
    readonly room: Address | null;
    // The first and last moments the page knows of, in ms.
    readonly from: number | null;
    readonly to: number | null;
  }

  const { turns, doing, closing, room, from, to }: Props = $props();
  const lang = ui().lang;
  const figures = $derived(figuresOf(turns));
  const models = $derived(modelsOf(turns));
</script>

<dl
  class="flex flex-wrap items-baseline gap-x-wide gap-y-snug border-y border-edge py-snug text-note"
  aria-label={say($lang, "run_head_region")}
>
  <div class="flex min-w-0 flex-wrap items-baseline gap-x-snug">
    <dt class="text-text-quiet">{say($lang, "run_head_outcome")}</dt>
    <dd class="text-text">{say($lang, POSTURE_WORD[doing?.kind ?? "unknown"])}</dd>
    {#if closing !== null && closing.completion !== ""}
      <dd class="truncate text-text-quiet">{closing.completion}</dd>
    {/if}
  </div>
  {#if from !== null && to !== null}
    <div class="flex min-w-0 flex-wrap items-baseline gap-x-snug">
      <dt class="text-text-quiet">{say($lang, "run_head_took")}</dt>
      <dd class="font-mono text-text">{lasted(to - from)}</dd>
      <dd class="text-text-quiet">{fill(say($lang, "run_head_started"), { clock: clock($lang, from) })}</dd>
    </div>
  {/if}
  <div class="flex min-w-0 flex-wrap items-baseline gap-x-snug">
    <dt class="text-text-quiet">{say($lang, "run_head_tokens")}</dt>
    <dd class="font-mono text-text">{count(figures.input)} / {count(figures.output)}</dd>
    <dd class="text-text-quiet">{fill(say($lang, "run_head_cached"), { n: count(figures.cached) })}</dd>
  </div>
  <div class="flex min-w-0 flex-wrap items-baseline gap-x-snug">
    <dt class="text-text-quiet">{say($lang, "run_head_spent")}</dt>
    <dd class="font-mono text-text">{usd(figures.usd)}</dd>
    <dd class="text-text-quiet">{fill(say($lang, "run_head_turns"), { n: String(turns.length) })}</dd>
  </div>
  {#if models.length > 0}
    <div class="flex min-w-0 flex-wrap items-baseline gap-x-snug">
      <dt class="text-text-quiet">{say($lang, "run_head_model")}</dt>
      {#each models as model (model)}
        <dd class="min-w-0 truncate font-mono text-text">{model}</dd>
      {/each}
    </div>
  {/if}
  {#if room !== null}
    <div class="flex min-w-0 flex-wrap items-baseline gap-x-snug">
      <dt class="text-text-quiet">{say($lang, "run_head_origin")}</dt>
      <dd class="min-w-0 truncate font-mono">
        <a href={toFragment({ kind: "building", address: buildingOf(room) })} class="text-text-faint"
          >{buildingOf(room)}</a
        >
        <span class="text-text-disabled">/</span>
        <a href={toFragment({ kind: "talk", address: room })} class="text-text">{roomOf(room)}</a>
      </dd>
    </div>
  {/if}
</dl>
