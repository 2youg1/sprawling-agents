<!-- This Source Code Form is subject to the terms of the Mozilla Public
License, v. 2.0. If a copy of the MPL was not distributed with this
file, You can obtain one at https://mozilla.org/MPL/2.0/.
Copyright (c) 2026 2youg1 and the sprawling contributors -->

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

    readonly turn: Turn;

    readonly onFork?: ((plan: ForkPlan) => void) | undefined;

    readonly onHover: (entry: ForkEntry | null) => void;

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
