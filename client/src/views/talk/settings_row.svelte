<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts" module>
  // How much of the row a composer draws (docs/frontend-method.md §7D, §7I): every
  // fact and choice before a session begins; the room, gate and sandbox
  // once it has, because the model, effort and mode are then the frozen
  // facts of its first message head; and in the panorama tier's band
  // none of them, because the chosen session's sheet says all of them.
  // A message the link would not take is said in every case.
  export type RowDraws = "everything" | "facts" | "notice";
</script>

<script lang="ts">
  // The row under the composer's line (docs/frontend-method.md §7I): the room, the gate
  // and the sandbox on the left, and - only before a session begins - the
  // model, the effort, the mode and the write limit on the right; the
  // write limit's menu also holds the admission requirement and the
  // landing (refrain roadmap 4-2). Once a session begins all of these are
  // frozen facts of the run, and the row stops offering them. The room
  // chip's menu reads who is listening above the rooms it offers.
  import { untrack } from "svelte";

  import { say } from "../../core/lang";
  import { ui } from "../../ui";
  import type { Address } from "../../wire";
  import Bounds from "./bounds.svelte";
  import type { Pill } from "./composer";
  import Listening from "./listening.svelte";
  import { Popover } from "../parts/popover";
  import type { PopoverColumn, PopoverRow } from "../parts/popover";
  import PillView, { FACT } from "./pill.svelte";
  import { picked, policyColumns, policyFace } from "./policy";

  interface Props {
    // The four pills `composer.ts` builds: model, room, effort, mode.
    readonly specs: readonly [Pill, Pill, Pill, Pill];
    readonly room: Address | null;
    readonly draws: RowDraws;
    // Whether the last message stayed in the box because the link would
    // not take it.
    readonly kept: boolean;
    // The gallery draws the run policy's menu open so it is measured;
    // every composer starts with it closed.
    readonly menu?: "open" | "closed";
  }

  const { specs, room, draws, kept, menu: starts = "closed" }: Props = $props();
  const u = ui();
  const { lang } = u;
  const policy = u.policy;
  let menu = $state(untrack(() => starts) === "open");

  function apply(column: PopoverColumn, row: PopoverRow): void {
    u.choosePolicy(picked($policy, column.id, row.id));
  }
</script>

{#snippet listening()}
  {#if room !== null}<Listening {room} />{/if}
{/snippet}

{#if draws !== "notice" || kept}
<!-- The run policy's menu hangs from the whole row, so its three columns
have the row's width. It stays open while the person picks in more than
one column, and Escape or its control closes it. -->
<div class="relative mt-tight flex flex-wrap items-center justify-between gap-tight">
  {#if menu && draws === "everything"}
    <Popover
      label="talk_policy"
      columns={policyColumns($lang, $policy)}
      onApply={apply}
      onClose={() => {
        menu = false;
      }}
    />
  {/if}
  <!-- On one column the edge keys stand at the start of this group, and
  its chips flow beside them (`edge-slot`, client/Spec.lean §4-52). -->
  <div class="edge-slot -ml-snug flex min-w-0 flex-wrap items-center narrow:ml-0">
    {#if draws !== "notice"}
      <PillView spec={specs[1]} told={room === null ? undefined : listening} />
      <Bounds {room} />
    {/if}
    {#if kept}
      <span class="px-snug text-note text-alert">{say($lang, "talk_not_live")}</span>
    {/if}
  </div>
  {#if draws === "everything"}
    <div class="-mr-snug ml-auto flex min-w-0 flex-wrap items-center narrow:mr-0">
      <PillView spec={specs[0]} />
      <PillView spec={specs[2]} />
      <PillView spec={specs[3]} />
      <button
        type="button"
        class="{FACT} hover:wash hover:text-text aria-expanded:wash aria-expanded:text-text"
        aria-label={`${say($lang, "talk_policy")}: ${policyFace($lang, $policy)}`}
        aria-haspopup="dialog"
        aria-expanded={menu}
        onclick={() => {
          menu = !menu;
        }}
      >
        <span class="truncate">{policyFace($lang, $policy)}</span>
      </button>
    </div>
  {/if}
</div>
{/if}
