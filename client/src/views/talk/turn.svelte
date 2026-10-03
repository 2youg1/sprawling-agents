<!-- This Source Code Form is subject to the terms of the Mozilla Public
License, v. 2.0. If a copy of the MPL was not distributed with this
file, You can obtain one at https://mozilla.org/MPL/2.0/.
Copyright (c) 2026 2youg1 and the sprawling contributors -->

<!-- One round of the model, in the order the template sets a message:
the head, what it reasoned (folded), what it said, and the calls it made
under that (refrain §3-2). Words that arrived from a person stand before
the round they arrived in; a refusal or a wait on the person after it.

**The block names its phase for the read-wear bar** (`data-wear`): a
round that waited on the person, one that worked through tools, or one
that only spoke. `wear.svelte` reads the attribute off the page, so the
bar and the thread cannot disagree about where a round is. -->
<script lang="ts">
  import { fill, say } from "../../core/lang";
  import { count } from "../../core/time";
  import type { Doing } from "../../core/doing";
  import type { RunId, Turn } from "../../wire";
  import { ui } from "../../ui";
  import Prose from "../prose.svelte";
  import type { Phase } from "../runs/lineage";
  import { arrivalsOf } from "./arrivals.svelte";
  import Calls from "./calls.svelte";
  import ForkButton from "./fork_button.svelte";
  import Head from "./head.svelte";
  import NoteLine from "./note_line.svelte";
  import { noteAt } from "./note_line";
  import { rhythmOf } from "./rhythm";
  import { cutOff, silentTurn } from "./silence";
  import type { Phase as Live } from "./silence";
  import type { ForkEntry, ForkPlan } from "./forking";
  import { tpsOf, ttftTookOf } from "./timing";

  interface Props {
    readonly turn: Turn;
    readonly run: RunId;
    readonly who: string;
    // The model this round's head states, or `null` when an earlier head
    // on this screen already said it.
    readonly model: string | null;
    readonly live: Live;
    // The run's posture, handed only to the round the run is in now.
    readonly doing: Doing | undefined;
    // Whether a round that said nothing draws its own warning; a run
    // that said nothing at all draws one card for all of them instead.
    readonly showEmpty: boolean;
    // What this city lets a reply be at most, when it says.
    readonly ceiling: number | null;
    // Whether the calls and the reasoning are drawn at all (`results`
    // draws neither).
    readonly whole: boolean;
    readonly onFork?: ((plan: ForkPlan) => void) | undefined;
    readonly onCall?: ((entry: ForkEntry) => void) | undefined;
    readonly onHover: (entry: ForkEntry | null) => void;
  }

  const { turn, run, who, model, live, doing, showEmpty, ceiling, whole, onFork, onCall, onHover }: Props = $props();

  const { lang } = ui();

  const said = $derived(turn.said ?? "");
  const empty = $derived(silentTurn(turn, live));
  const cut = $derived(cutOff(turn.stopped));
  const phase = $derived.by((): Phase => {
    if (turn.notes.some((note) => "waiting" in note)) return "person";
    return turn.calls.length > 0 ? "tool" : "model";
  });
  const arrivals = $derived(arrivalsOf(run, turn.opened));
  const rhythm = $derived(arrivals === null ? null : rhythmOf(arrivals));
</script>

<div class="group relative flex flex-col gap-snug pb-section last:pb-0" data-wear={phase}>
  {#if onFork !== undefined}
    <ForkButton entry={{ kind: "turn", turn }} {run} {onFork} {onHover} />
  {/if}
  {#each turn.notes.filter((note) => "arrived" in note) as note (noteAt(note))}
    <NoteLine {note} {turn} {run} {onFork} {onHover} />
  {/each}
  {#if said !== "" || model !== null}
    <Head
      {who}
      at={turn.t}
      {model}
      ttft={said === "" ? null : ttftTookOf(turn)}
      tps={said === "" ? null : tpsOf(turn)}
      {rhythm}
    />
  {/if}
  {#if turn.thought && whole}
    <details class="text-note text-text-faint">
      <summary class="cursor-pointer rounded-control px-tight marker:text-text-faint hover:bg-chrome hover:text-text-quiet">
        <span class="text-text-faint">{say($lang, "talk_reasoning")}</span>
        {fill(say($lang, "talk_reasoning_length"), { n: count(turn.thought.length) })}
      </summary>
      <div class="mt-tight border-l border-edge-panel pl-base whitespace-pre-wrap break-words">{turn.thought}</div>
    </details>
  {/if}
  {#if said !== ""}
    <div class="text-body"><Prose text={said} /></div>
  {/if}
  {#if turn.calls.length > 0 && whole}
    <Calls calls={turn.calls} {run} {turn} {doing} onFork={onCall} />
  {/if}
  {#if empty && showEmpty}
    <div class="rounded-card border border-alert/40 px-base py-snug text-note text-alert">
      {ceiling === null
        ? say($lang, "talk_said_nothing")
        : fill(say($lang, "talk_said_nothing_capped"), { n: count(ceiling) })}
    </div>
  {/if}
  {#if cut !== null}
    <div class="text-note text-alert">{fill(say($lang, "talk_cut_off"), { why: cut })}</div>
  {/if}
  {#each turn.notes.filter((note) => !("arrived" in note)) as note (noteAt(note))}
    <NoteLine {note} {turn} {run} {onFork} {onHover} />
  {/each}
</div>
