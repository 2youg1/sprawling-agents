<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  import { fill, say } from "../../core/lang";
  import { toFragment } from "../../core/route";
  import { count } from "../../core/time";
  import type { Call, Output, RunId, Turn } from "../../wire";
  import { ui } from "../../ui";
  import { callWord } from "./calls";
  import type { ForkEntry } from "./forking";
  import { tally } from "./trace";

  interface Props {
    readonly calls: readonly Call[];
    readonly run: RunId;
    // The turn these calls belong to: a branch from one of them names
    // the turn it is inside, and the walk-back needs where the turn
    // opened.
    readonly turn: Turn;
    // Absent where the page holding the thread cannot branch - the run
    // page shows the same fold with nowhere to fork to.
    readonly onFork?: ((entry: ForkEntry) => void) | undefined;
  }

  const { calls, run, turn, onFork }: Props = $props();

  const { lang } = ui();

  // One clause per class of work that happened, and none for a class
  // that did not: "explored 0 files" is a sentence about nothing.
  const summary = $derived.by(() => {
    const counted = tally(calls);
    const said: string[] = [];
    if (counted.explored > 0) said.push(fill(say($lang, "talk_deed_explored"), { n: count(counted.explored) }));
    if (counted.wrote > 0) said.push(fill(say($lang, "talk_deed_wrote"), { n: count(counted.wrote) }));
    if (counted.ran > 0) said.push(fill(say($lang, "talk_deed_ran"), { n: count(counted.ran) }));
    if (counted.other > 0) said.push(fill(say($lang, "talk_deed_other"), { n: count(counted.other) }));
    return said.join(" · ");
  });
</script>

<!-- What a call was given and what it said, cut the same way: the
     arguments of a write can be as long as its output, and the cut is
     counted either way. -->
{#snippet bounded(output: Output)}
  <pre
    class="mt-tight max-h-output overflow-auto rounded-card border border-edge bg-page p-snug font-mono text-note text-text-quiet"
  >{output.cut > 0 ? `${output.head}\n` : output.head}{#if output.cut > 0}<span class="text-text-disabled">{fill(say($lang, "run_cut"), { n: String(output.cut) })}</span>{/if}</pre>
  {#if output.cut > 0}
    <a class="text-note text-text-faint hover:text-text-quiet" href={toFragment({ kind: "run", run })}>
      {say($lang, "talk_call_open")}
    </a>
  {/if}
{/snippet}

<!-- What a turn did, folded to one line.

     A wave of tool calls is the bulk of what a run produces and almost
     none of what a person reads a thread for, so the thread states the
     shape of the wave - explored so many files, wrote so many, ran so
     many commands - and opens to the calls themselves on a click.

     **The fold is a `<details>`.** Opening, closing, the disclosure
     triangle, the keyboard and the state a screen reader reports are the
     element's, and the height travels because `theme.css` declares
     `interpolate-size` once for the whole page. An engine that has
     neither opens the fold at once, which is the behaviour that engine
     has today and not a branch anybody writes.

     Each call carries the branch action the thread gives every entry:
     a call is a place the conversation could have gone another way
     (roadmap S2). -->

<details class="my-tight text-note text-text-faint">
  <summary
    class="cursor-pointer rounded-control px-tight marker:text-text-disabled hover:bg-chrome hover:text-text-quiet"
  >
    {summary}
  </summary>
  <ul class="mt-tight ml-pane border-l border-edge pl-base">
    {#each calls as call (call.at)}
      <li class="group relative my-tight">
        {#if onFork !== undefined}
          <button
            type="button"
            class="absolute top-0 right-0 rounded-control px-tight text-note text-text-faint opacity-0 transition-opacity group-focus-within:opacity-100 group-hover:opacity-100 focus:opacity-100 hover:bg-chrome hover:text-text-quiet"
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
        <span
          class={call.outcome === "failed"
            ? "text-alert"
            : call.outcome === "waiting"
              ? "animate-pulse"
              : ""}
        >
          {callWord(call.tool, call.subject)}
        </span>
        {#if call.arguments}
          <span class="mt-tight block text-text-disabled">{say($lang, "talk_call_arguments")}</span>
          <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression (a snippet call is the render itself) -->
          {@render bounded(call.arguments)}
        {/if}
        {#if call.output}
          <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression (a snippet call is the render itself) -->
          {@render bounded(call.output)}
        {/if}
      </li>
    {/each}
  </ul>
</details>
