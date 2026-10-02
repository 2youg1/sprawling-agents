<!-- This Source Code Form is subject to the terms of the Mozilla Public
License, v. 2.0. If a copy of the MPL was not distributed with this
file, You can obtain one at https://mozilla.org/MPL/2.0/.
Copyright (c) 2026 2youg1 and the sprawling contributors -->

<!-- The calls a turn made, one line each, under what the turn said.

A wave of calls is most of what a run produces and little of what a
person reads a thread for, so each call is one quiet line - what kind of
thing it did, on what, how long it took, and how it came out - and the
whole of it opens on the right side (refrain §3-4, Q2). What a call was
given and what it printed belong there, not under the line: the thread
says the shape of the work, the right side shows the work.

Each call carries the branch action the thread gives every entry, because
a call is a place the conversation could have gone another way (roadmap
S2). -->
<script lang="ts">
  import { say } from "../../core/lang";
  import type { Doing } from "../../core/doing";
  import type { Call, RunId, Turn } from "../../wire";
  import { ui } from "../../ui";
  import CallLine from "./call_line.svelte";
  import type { ForkEntry } from "./forking";

  interface Props {
    readonly calls: readonly Call[];
    readonly run: RunId;
    // The turn these calls belong to: a branch from one of them names
    // the turn it is inside, and the walk-back needs where the turn
    // opened.
    readonly turn: Turn;
    // Absent where the page holding the thread cannot branch - the run
    // page shows the same lines with nowhere to fork to.
    readonly onFork?: ((entry: ForkEntry) => void) | undefined;
    // The run's posture, handed only to the turn the run is in now.
    readonly doing?: Doing | undefined;
  }

  const { calls, run, turn, onFork, doing }: Props = $props();

  const { lang } = ui();
</script>

<ul class="my-tight flex flex-col" aria-label={say($lang, "talk_calls")}>
  {#each calls as call (call.at)}
    <li class="group flex items-center [&>span]:flex-1">
      <CallLine {call} {run} {doing} />
      {#if onFork !== undefined}
        <button
          type="button"
          class="h-control w-control shrink-0 rounded-control text-note text-text-faint opacity-0 transition-opacity group-focus-within:opacity-100 group-hover:opacity-100 focus:opacity-100 hover:bg-chrome hover:text-text-quiet"
          aria-label={say($lang, "fork_here")}
          onclick={() => {
            onFork({ kind: "call", turn, call });
          }}
        >
          <!-- wording-ok: a drawing in type, not a word; the action's
               name is the aria-label above. -->
          ⑂
        </button>
      {/if}
    </li>
  {/each}
</ul>
