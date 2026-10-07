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
  import ForkButton from "./fork_button.svelte";
  import type { ForkEntry, ForkPlan } from "./forking";

  interface Props {
    readonly calls: readonly Call[];
    readonly run: RunId;
    // The turn these calls belong to: a branch from one of them names
    // the turn it is inside, and the walk-back needs where the turn
    // opened.
    readonly turn: Turn;
    // Absent where the page holding the thread cannot branch - the run
    // page shows the same lines with nowhere to branch to.
    readonly onFork?: ((plan: ForkPlan) => void) | undefined;
    // The entry under the hand, which the `fork.here` chord branches
    // from: a call is one, like a reply or a person's words.
    readonly onHover: (entry: ForkEntry | null) => void;
    // The run's posture, handed only to the turn the run is in now.
    readonly doing?: Doing | undefined;
  }

  const { calls, run, turn, onFork, onHover, doing }: Props = $props();

  const { lang } = ui();
</script>

<ul class="my-tight flex flex-col" aria-label={say($lang, "talk_calls")}>
  {#each calls as call (call.at)}
    <li class="group flex items-center gap-tight [&>span:first-child]:flex-1">
      <CallLine {call} {run} {doing} />
      {#if onFork !== undefined}
        <ForkButton entry={{ kind: "call", turn, call }} {run} {onFork} {onHover} />
      {/if}
    </li>
  {/each}
</ul>
