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
says the shape of the work, the right side shows the work. -->
<script lang="ts">
  import { say } from "../../core/lang";
  import type { Doing } from "../../core/doing";
  import type { Call, RunId } from "../../wire";
  import { ui } from "../../ui";
  import CallLine from "./call_line.svelte";

  interface Props {
    readonly calls: readonly Call[];
    readonly run: RunId;
    // The run's posture, handed only to the turn the run is in now.
    readonly doing?: Doing | undefined;
  }

  const { calls, run, doing }: Props = $props();

  const { lang } = ui();
</script>

<ul class="my-tight flex flex-col" aria-label={say($lang, "talk_calls")}>
  {#each calls as call (call.at)}
    <li class="flex items-center [&>span:first-child]:flex-1">
      <CallLine {call} {run} {doing} />
    </li>
  {/each}
</ul>
