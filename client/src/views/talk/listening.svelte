<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  // Who is listening in this room (docs/frontend-method.md §7I), read in the room
  // chip's menu above the rooms it offers: the city, the building, the
  // resident and the run, one row each, and beside each the segment of
  // the prompt that the room's newest run was told for it - its size,
  // the files it was read from, and what the budget cut off them.
  //
  // It is the run page's prompt lens (`run/prompt.svelte`) cut to what a
  // person checks before writing - who hears this and from which files -
  // and it reads the same answer (`Query::Prefix`). The text itself stays
  // on the run page: a menu over the box is no place to read four
  // documents, and it holds no control of its own, because the menu's
  // keys belong to its list.
  import { fill, say } from "../../core/lang";
  import type { Key } from "../../core/lang";
  import { buildingOf } from "../../core/route";
  import { count } from "../../core/time";
  import { ui } from "../../ui";
  import type { Address, PrefixSegment, PrefixSlot } from "../../wire";
  import { sessionRunOf } from "./gauge";

  interface Props {
    readonly room: Address;
  }

  const { room }: Props = $props();

  const u = ui();
  const { lang } = u;
  const belief = u.conn.belief;

  const run = $derived(sessionRunOf($belief, room));
  let segments = $state<readonly PrefixSegment[]>([]);
  $effect(() => {
    const at = run;
    segments = [];
    if (at === undefined) return;
    return u.conn.asking.ask({ prefix: { run: at } }).subscribe((held) => {
      segments = held !== undefined && "prefix" in held ? held.prefix.segments : [];
    });
  });

  interface Listener {
    readonly slot: PrefixSlot;
    readonly word: Key;
    readonly who: string;
  }

  // The four listeners in the order a prompt stacks their segments. The
  // run has no name beyond itself, which the conversation already is.
  const listeners = $derived<readonly Listener[]>([
    { slot: "city", word: "slot_city", who: $belief.city ?? "" },
    { slot: "building", word: "slot_building", who: buildingOf(room) },
    { slot: "resident", word: "slot_resident", who: room },
    { slot: "run", word: "slot_run", who: "" },
  ]);
</script>

<div class="px-base pb-snug text-note">
  <p class="pb-tight text-text-faint">{say($lang, "talk_listening")}</p>
  <dl class="grid grid-cols-[max-content_minmax(0,1fr)] gap-x-base gap-y-tight">
    {#each listeners as listener (listener.slot)}
      {@const told = segments.find((each) => each.slot === listener.slot)}
      {@const cut = told === undefined ? 0 : told.sources.reduce((sum, source) => sum + source.dropped, 0)}
      <dt class="text-text-faint">{say($lang, listener.word)}</dt>
      <dd class="flex min-w-0 flex-col">
        <span class="flex min-w-0 items-baseline gap-base">
          {#if listener.who !== ""}<span class="truncate text-text">{listener.who}</span>{/if}
          {#if told !== undefined}
            <span class="figure shrink-0 text-text-faint">{fill(say($lang, "run_prompt_bytes"), { n: count(told.bytes) })}</span>
          {/if}
          {#if cut > 0}
            <span class="shrink-0 text-alert">{fill(say($lang, "run_prompt_dropped"), { n: count(cut) })}</span>
          {/if}
        </span>
        {#if told !== undefined && told.sources.length > 0}
          <span class="truncate text-text-faint">
            {say($lang, "run_prompt_sources")}
            {told.sources.map((source) => source.addr).join(" · ")}
          </span>
        {/if}
      </dd>
    {/each}
  </dl>
  {#if run !== undefined}
    <p class="pt-tight text-text-faint">{say($lang, "talk_listening_note")}</p>
  {/if}
</div>
