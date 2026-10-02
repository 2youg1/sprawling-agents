<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  // An earlier session of a room, in main (client/Spec.lean §7K): its runs as
  // the conversation draws them, and no box under them, because the city
  // goes on only with a room's current session. In the box's place stands
  // what can be done instead: continue from the session's end, which is a
  // branch from its tail (the frame `/fork` sends), or go back to the
  // room's current session. A turn's own fork key branches from that
  // turn the same way. Either branch brings main to the room's current
  // session, which the branch has just become.
  //
  // In the panorama tier's band only the offer stands; the session pane
  // above it shows the session.
  import { openSession } from "../../core/commands";
  import { fill, say } from "../../core/lang";
  import { roomOf, toFragment } from "../../core/route";
  import { tailOf } from "../../core/stretches";
  import type { Stretch } from "../../core/stretches";
  import { ui } from "../../ui";
  import type { Address, Origin } from "../../wire";
  import Button from "../parts/button.svelte";
  import Scroller from "./scroller.svelte";
  import type { ForkPlan } from "./forking";
  import Thread from "./thread.svelte";

  interface Props {
    readonly address: Address;
    // The session the address bar names; null while the city has not
    // answered for it, or no longer lists it.
    readonly stretch: Stretch | null;
    readonly band: boolean;
  }

  const { address, stretch, band }: Props = $props();

  const u = ui();
  const { lang } = u;
  const who = $derived(roomOf(address));
  const tail = $derived(stretch === null ? null : tailOf(stretch));

  function branch(origin: Origin): void {
    if (u.send(openSession(address, "nothing", origin))) u.go({ kind: "talk", address });
  }
</script>

<div class={["flex min-h-0 flex-col", band ? "" : "h-full"]}>
  {#if !band}
    <Scroller empty={false} rejoined={0} place={stretch === null ? address : `${address}:${String(stretch.line.began)}`}>
      {#each stretch?.runs ?? [] as run, at (run.run)}
        <Thread
          {run}
          opens={at === 0}
          onFork={(plan: ForkPlan) => {
            branch(plan.origin);
          }}
        />
      {/each}
    </Scroller>
  {/if}
  <div class="flex shrink-0 flex-col gap-snug border-t border-edge-panel pt-base">
    <p class="text-note text-text-quiet">{fill(say($lang, "talk_past"), { room: who })}</p>
    <div class="flex flex-wrap items-center gap-base">
      {#if tail === null}
        <Button label={say($lang, "talk_continue")} tone="secondary" why={say($lang, "talk_continue_unheld")} />
      {:else}
        <Button
          label={say($lang, "talk_continue")}
          tone="secondary"
          onPress={() => {
            branch(tail);
          }}
        />
      {/if}
      <a class="text-note text-text-quiet underline-offset-2 hover:text-text hover:underline" href={toFragment({ kind: "talk", address })}>
        {say($lang, "talk_current")}
      </a>
    </div>
  </div>
</div>
