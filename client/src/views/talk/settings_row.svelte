<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts" module>
  // How much of the row a composer draws (client-SPEC 7D, 7I): every
  // fact and choice before a session begins; the room, gate and sandbox
  // once it has, because the model, effort and mode are then the frozen
  // facts of its first message head; and in the panorama tier's band
  // none of them, because the chosen session's sheet says all of them.
  // A message the link would not take is said in every case.
  export type RowDraws = "everything" | "facts" | "notice";
</script>

<script lang="ts">
  // The row under the composer's line (client-SPEC 7I): the room, the gate
  // and the sandbox on the left, and - only before a session begins - the
  // model, the effort and the mode on the right. The room chip's menu
  // reads who is listening above the rooms it offers.
  import { say } from "../../core/lang";
  import { ui } from "../../ui";
  import type { Address } from "../../wire";
  import Bounds from "./bounds.svelte";
  import type { Pill } from "./composer";
  import Listening from "./listening.svelte";
  import PillView from "./pill.svelte";

  interface Props {
    // The four pills `composer.ts` builds: model, room, effort, mode.
    readonly specs: readonly [Pill, Pill, Pill, Pill];
    readonly room: Address | null;
    readonly draws: RowDraws;
    // Whether the last message stayed in the box because the link would
    // not take it.
    readonly kept: boolean;
  }

  const { specs, room, draws, kept }: Props = $props();
  const { lang } = ui();
</script>

{#snippet listening()}
  {#if room !== null}<Listening {room} />{/if}
{/snippet}

{#if draws !== "notice" || kept}
<div class="mt-tight flex flex-wrap items-center justify-between gap-tight">
  <div class="-ml-snug flex min-w-0 flex-wrap items-center narrow:ml-0">
    {#if draws !== "notice"}
      <PillView spec={specs[1]} told={room === null ? undefined : listening} />
      <Bounds {room} />
    {/if}
    {#if kept}
      <span class="px-snug text-note text-alert">{say($lang, "talk_not_live")}</span>
    {/if}
  </div>
  {#if draws === "everything"}
    <div class="-mr-snug flex min-w-0 flex-wrap items-center narrow:mr-0">
      <PillView spec={specs[0]} />
      <PillView spec={specs[2]} />
      <PillView spec={specs[3]} />
    </div>
  {/if}
</div>
{/if}
