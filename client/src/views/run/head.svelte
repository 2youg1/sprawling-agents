<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // The run page's sheet: how the run stands, how long it took and when
  // it began, what it read and wrote in tokens, what it cost, which
  // model answered, where it lives and who sent it - each fact a cell of
  // one grid, its name small above its value, the way the panorama's
  // session sheet sets them. Every figure is summed from the rounds the
  // page already asked for, so the sheet and the lenses under it cannot
  // disagree, and each fact is written here once: the lenses below do
  // not repeat a total.

  import type { Doing } from "../../core/doing";
  import { POSTURE_WORD } from "../../core/doing";
  import { fill, say } from "../../core/lang";
  import { buildingOf, roomOf, toFragment } from "../../core/route";
  import { count, isoDay, isoTime, lasted, usd } from "../../core/time";
  import { ui } from "../../ui";
  import type { Address, Closing, Turn } from "../../wire";
  import { figuresOf, modelsOf } from "./lanes";

  interface Props {
    readonly turns: readonly Turn[];
    readonly doing: Doing | undefined;
    readonly closing: Closing | null;
    readonly room: Address | null;
    // Who sent the run, as its run_started records it: `person`, `city`
    // or the address of the resident that did; null when the ledger
    // does not say.
    readonly dispatchedBy: string | null;
    // The first and last moments the page knows of, in ms.
    readonly from: number | null;
    readonly to: number | null;
  }

  const { turns, doing, closing, room, dispatchedBy, from, to }: Props = $props();
  const lang = ui().lang;
  const figures = $derived(figuresOf(turns));
  const models = $derived(modelsOf(turns));

  const CELL = "flex min-w-0 flex-col";
  const NAME = "text-note text-text-faint";
  const VALUE = "figure truncate text-text";
  const MORE = "truncate text-note text-text-quiet";
</script>

<dl
  class="grid grid-cols-[repeat(auto-fill,minmax(22ch,1fr))] gap-x-gutter gap-y-base border-y border-edge py-base"
  aria-label={say($lang, "run_head_region")}
>
  <div class={CELL}>
    <dt class={NAME}>{say($lang, "run_head_outcome")}</dt>
    <dd class="truncate text-text">{say($lang, POSTURE_WORD[doing?.kind ?? "unknown"])}</dd>
    {#if closing !== null && closing.completion !== ""}
      <dd class={MORE}>{closing.completion}</dd>
    {/if}
  </div>
  {#if from !== null && to !== null}
    <!-- Two columns wide: the start is a whole ISO instant, and cutting
         it short would cut the part that says when. -->
    <div class="{CELL} col-span-2">
      <dt class={NAME}>{say($lang, "run_head_took")}</dt>
      <dd class={VALUE}>{lasted(to - from)}</dd>
      <dd class="{MORE} figure">
        <time datetime={`${isoDay(from)}T${isoTime(from)}`}
          >{fill(say($lang, "run_head_started"), { clock: `${isoDay(from)} ${isoTime(from)}` })}</time
        >
      </dd>
    </div>
  {/if}
  <div class={CELL}>
    <dt class={NAME}>{say($lang, "run_head_tokens")}</dt>
    <dd class={VALUE}>{count(figures.input)} / {count(figures.output)}</dd>
    <dd class="{MORE} figure">{fill(say($lang, "run_head_cached"), { n: count(figures.cached) })}</dd>
  </div>
  <div class={CELL}>
    <dt class={NAME}>{say($lang, "run_head_spent")}</dt>
    <dd class={VALUE}>{usd(figures.usd)}</dd>
    <dd class="{MORE} figure">{fill(say($lang, "run_head_turns"), { n: String(turns.length) })}</dd>
  </div>
  {#if models.length > 0}
    <div class={CELL}>
      <dt class={NAME}>{say($lang, "run_head_model")}</dt>
      {#each models as model (model)}
        <dd class="truncate font-mono text-text">{model}</dd>
      {/each}
    </div>
  {/if}
  {#if room !== null}
    <div class={CELL}>
      <dt class={NAME}>{say($lang, "run_head_origin")}</dt>
      <dd class="truncate font-mono">
        <a href={toFragment({ kind: "building", address: buildingOf(room) })} class="text-text-quiet hover:text-text"
          >{buildingOf(room)}</a
        ><span class="text-text-faint">/</span><a href={toFragment({ kind: "talk", address: room })} class="text-text"
          >{roomOf(room)}</a
        >
      </dd>
    </div>
  {/if}
  {#if dispatchedBy !== null}
    <div class={CELL}>
      <dt class={NAME}>{say($lang, "run_head_dispatched_by")}</dt>
      <dd class="truncate font-mono text-text">{dispatchedBy}</dd>
    </div>
  {/if}
</dl>
