<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  // The third line of a sessions row (client-SPEC 7K): a 2 px bar of how
  // much of the window the room's newest run used, with a tick where the
  // handoff reminder sounds, so a person scanning the pane sees which
  // session is about to hand off. It is the context ring's reading drawn
  // flat: the same pair of numbers and the same settled threshold, read
  // through `talk/gauge.ts`, so a row and the ring cannot disagree.
  //
  // A run whose model stated no window draws no bar, as the ring draws
  // no ring. The share is said to a screen reader as words inside the
  // row's link; the bar itself is drawing.
  import { QUERIES } from "../../core/asking";
  import { fill, say } from "../../core/lang";
  import { ui } from "../../ui";
  import type { Address, RoundsAnswer, RunId } from "../../wire";
  import { contextOf, remindersOf, usedPercent } from "../talk/gauge";

  interface Props {
    readonly room: Address;
    readonly run: RunId;
  }

  const { room, run }: Props = $props();

  const u = ui();
  const { lang } = u;
  const endpoints = u.conn.asking.ask(QUERIES.endpoints);
  const config = $derived(u.conn.asking.ask({ config: { addr: room } }));

  let rounds = $state<RoundsAnswer | undefined>(undefined);
  $effect(() => {
    const at = run;
    rounds = undefined;
    return u.conn.asking.ask({ rounds: { run: at } }).subscribe((held) => {
      rounds = held !== undefined && "rounds" in held ? held.rounds : undefined;
    });
  });

  const context = $derived(
    contextOf(
      rounds?.turns ?? [],
      $endpoints !== undefined && "endpoints" in $endpoints ? $endpoints.endpoints.endpoints : [],
      remindersOf($config),
    ),
  );
</script>

{#if context !== null}
  {@const share = usedPercent(context)}
  <span class="relative col-start-2 col-end-4 mt-snug h-hair rounded-pill bg-edge-panel" aria-hidden="true">
    <span class="absolute inset-y-0 left-0 rounded-pill bg-text-quiet" style:width="{share}%"></span>
    {#if context.second !== null}
      <span class="absolute -top-[2px] h-[6px] w-px bg-reminder-second" style:left="{context.second}%"></span>
    {/if}
  </span>
  <span class="sr-only">{fill(say($lang, "ring_share"), { n: String(share) })}</span>
{/if}
