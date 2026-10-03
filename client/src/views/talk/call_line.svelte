<!-- This Source Code Form is subject to the terms of the Mozilla Public
License, v. 2.0. If a copy of the MPL was not distributed with this
file, You can obtain one at https://mozilla.org/MPL/2.0/.
Copyright (c) 2026 2youg1 and the sprawling contributors -->

<!-- One tool call, pressed to one line: `kind  subject  time  result`
(refrain §3-4, client/Spec.lean §4-44). The line is the trigger of the right
side: pressing it shows the whole call there, through the one door every
opener uses (`inspect/open.svelte.ts`).

**The time cell has two readings and one clock.** While the call runs it
draws tenths from this page's ticker, recomputed from the call's own
moment on every tick; once the result is in the Ledger it draws the
Ledger's milliseconds, and the moment it finished is in the hint as an
ISO instant (docs/frontend-method.md §7D). A span nobody measured draws nothing.

**Keys belong to the line, not to the page.** The line walks by the one
table of line keys (`core/lines.ts`): ↑ and ↓ (and j and k) move to the
line above or below inside the same conversation, Home and End (and gg
and G) to the first and the last; Enter and Space press it; Escape puts
the right side away and keeps the focus here, so the person is still
standing where they were reading. The line that holds the focus, and the
line the right side shows, draw those keys at their end (refrain 3-12's
third layer). -->
<script lang="ts">
  import { fill, say } from "../../core/lang";
  import type { Doing } from "../../core/doing";
  import type { Call, RunId } from "../../wire";
  import { isoInstant } from "../../core/time";
  import { lineWalker } from "../../core/lines";
  import { pressedOf } from "../../core/press";
  import { ui } from "../../ui";
  import { Kbd } from "../parts/kbd.svelte";
  import Tip from "../parts/tip.svelte";
  import { closeRight, openCall, rightItem } from "../inspect/open.svelte";
  import { kindOf } from "./call_kind";
  import { NAMED_AFTER_MS, callTime, runningWords, tookWords, ticker } from "./timing";

  interface Props {
    readonly call: Call;
    readonly run: RunId;
    // The run's posture when this line belongs to the turn the run is in
    // now; absent for a turn that is over.
    readonly doing?: Doing | undefined;
  }

  const { call, run, doing }: Props = $props();

  const u = ui();
  const { lang } = u;
  const tick = ticker(u.now);

  const kind = $derived(kindOf(call));
  const time = $derived(callTime(call, call.outcome === "waiting" ? $tick : 0));
  const opened = $derived.by(() => {
    const item = rightItem();
    return item !== null && item.kind === "call" && item.run === run && item.at === call.at;
  });
  // A steer pressed now is heard after this call: the pin stands on the
  // line the run is actually in (refrain §3-5).
  const pinned = $derived(call.outcome === "waiting" && doing?.kind === "calling");
  // Past ten seconds a running line says what it waits for, and only when
  // that is known: a run waiting on the person says so; anything else
  // keeps its silence rather than invent a reason.
  const waitsForYou = $derived(
    call.outcome === "waiting" && doing?.kind === "waiting" && time.kind === "running" && time.ms >= NAMED_AFTER_MS,
  );
  const hint = $derived(
    call.answered === null || call.answered === undefined
      ? fill(say($lang, "talk_call_started"), { at: isoInstant(call.called) })
      : fill(say($lang, "talk_call_finished"), { at: isoInstant(call.answered) }),
  );

  const walk = lineWalker();
  // The keys the line draws at its end, in the order refrain 3-12 names
  // them: up, down, open, close.
  const MOVES_DRAWN = ["line.previous", "line.next", "line.open", "line.close"] as const;

  function linesBeside(from: HTMLElement): HTMLElement[] {
    const root = from.closest("[data-thread]") ?? document;
    return [...root.querySelectorAll<HTMLElement>("[data-call-line]")];
  }

  function onKeydown(event: KeyboardEvent): void {
    const line = event.currentTarget;
    if (!(line instanceof HTMLElement)) return;
    const lines = linesBeside(line);
    const at = lines.indexOf(line);
    const move = walk(pressedOf(event), event.timeStamp);
    const next = (() => {
      switch (move) {
        case "line.next":
          return lines[at + 1];
        case "line.previous":
          return lines[at - 1];
        case "line.first":
          return lines[0];
        case "line.last":
          return lines.at(-1);
        case "line.open":
        case "line.close":
        case null:
          return undefined;
      }
    })();
    if (move === "line.close" && opened) {
      event.preventDefault();
      closeRight();
    } else if (next !== undefined) {
      event.preventDefault();
      next.focus();
    }
  }
</script>

<Tip text={hint}>
  {#snippet children(id)}
    <button
      type="button"
      data-call-line=""
      aria-describedby={id}
      aria-pressed={opened}
      class={[
        "group relative grid h-control w-full grid-cols-[7ch_minmax(0,1fr)_9ch_auto] items-center gap-x-pane rounded-control px-snug text-left text-note narrow:grid-cols-[6ch_minmax(0,1fr)_auto_auto] narrow:gap-x-snug",
        "before:absolute before:inset-y-snug before:left-0 before:w-hair before:rounded-pill",
        opened ? "bg-raised text-text before:bg-accent" : "text-text-quiet hover:wash",
      ]}
      onclick={() => {
        openCall({ run, at: call.at });
      }}
      onkeydown={onKeydown}
    >
      <span class="truncate text-text-faint">
        {kind.kind === "registered" ? say($lang, kind.word) : kind.tool}
      </span>
      <span class={["truncate", call.outcome === "waiting" ? "text-text" : ""]}>{call.subject ?? ""}</span>
      <span class="figure flex items-center justify-end gap-tight whitespace-nowrap text-text-faint">
        {#if call.outcome === "waiting"}
          <span class="inline-block size-dot shrink-0 animate-pulse rounded-pill bg-accent" aria-hidden="true"></span>
        {/if}
        {#if time.kind === "landed"}
          {tookWords(time.took, $lang)}
        {:else if time.kind === "running"}
          <span class="text-text">{runningWords(time.ms)}</span>
        {/if}
      </span>
      <span class="flex items-center gap-tight">
        {#if call.outcome === "failed"}
          <span class="text-alert">{say($lang, "talk_call_failed")}</span>
        {:else if waitsForYou}
          <span class="text-alert">{say($lang, "talk_waiting_you")}</span>
        {/if}
        <span
          data-line-keys
          class={["items-center gap-hair narrow:hidden", opened ? "inline-flex" : "hidden group-focus-visible:inline-flex"]}
          aria-hidden="true"
        >
          {#each MOVES_DRAWN as move (move)}
            <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression, @typescript-eslint/no-unsafe-call (a snippet call is the render itself; typescript-eslint does not resolve exports of another .svelte module) -->
            {@render Kbd({ move })}
          {/each}
        </span>
        {#if pinned}
          <!-- The steer pin: a small accent wedge pointing at this line.
          It repeats what the coin's name already says (client/Spec.lean
          §4-13), so it is drawn for the eye and hidden from a reader. -->
          <span class="steer-pin" aria-hidden="true"></span>
        {/if}
      </span>
    </button>
  {/snippet}
</Tip>
