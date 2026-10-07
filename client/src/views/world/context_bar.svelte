<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  // The third line of a sessions row (client/Spec.lean §7K): a 2 px bar
  // of how much of the window the room's newest run used, with the
  // handoff checkpoint where that reminder sounds, so a person scanning
  // the pane sees which session is about to hand off. It is the context
  // ring's reading drawn
  // flat: the same pair of numbers, the same settled threshold and the
  // same checkpoint, read through `talk/gauge.ts`, so a row and the ring
  // cannot disagree.
  //
  // A run whose model stated no window draws no bar, as the ring draws
  // no ring. The share is said to a screen reader as words inside the
  // row's link; the bar itself is drawing (`context_bar.look.svelte`).
  //
  // The seat keeps the bar's place in the row's grid - the second and
  // third columns, a step below the line above - because the row lays
  // out its lines and the look draws inside the place it is given.
  import { QUERIES } from "../../core/asking";
  import { ui } from "../../ui";
  import type { Address, RoundsAnswer, RunId } from "../../wire";
  import { barOf, contextOf, remindersOf } from "../talk/gauge";
  import Look from "./context_bar.look.svelte";

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

  const bar = $derived(
    barOf(
      $lang,
      contextOf(
        rounds?.turns ?? [],
        $endpoints !== undefined && "endpoints" in $endpoints ? $endpoints.endpoints.endpoints : [],
        remindersOf($config),
      ),
    ),
  );
</script>

{#if bar !== null}
  <span class="col-start-2 col-end-4 mt-snug"><Look {...bar} /></span>
{/if}
