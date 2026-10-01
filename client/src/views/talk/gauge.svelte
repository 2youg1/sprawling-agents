<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  // The context ring round the coin key (client-SPEC 7J): where the eye
  // already is, without taking a place of its own. A whole one-pixel
  // ring is an empty window; the used part is eaten away clockwise from
  // twelve o'clock, and two checkpoints mark where the reminders sound.
  // The ring stands still while the key turns over inside it.
  //
  // It asks three questions the page already asks elsewhere, and
  // `asking` merges them by content: the session's rounds, the
  // endpoints with their models' windows, and the settled second
  // threshold for this room. What it draws from them is `gauge.ts`'s.
  import type { Snippet } from "svelte";

  import { QUERIES } from "../../core/asking";
  import { heldIn } from "../../core/belief/rooms";
  import { fill, say } from "../../core/lang";
  import { kilo } from "../../core/time";
  import { ui } from "../../ui";
  import type { Address, RoundsAnswer, RunId } from "../../wire";
  import Tip from "../parts/tip.svelte";
  import { contextOf, remainingArc, usedPercent } from "./gauge";

  interface Props {
    // The room the composer speaks to; its newest session is the one the
    // ring measures.
    readonly room: Address | null;
    readonly children: Snippet;
  }

  const { room, children: coin }: Props = $props();

  const u = ui();
  const { lang } = u;
  const belief = u.conn.belief;
  const endpoints = u.conn.asking.ask(QUERIES.endpoints);

  // The session's latest run, whose turns say how full its window is:
  // runs the room held before its newest session began belong to the
  // earlier stretch and are not measured.
  const run = $derived.by((): RunId | undefined => {
    if (room === null) return undefined;
    const began = $belief.sessions[room] ?? null;
    return heldIn($belief, room)
      .filter((each) => began === null || each.lastSeq > began)
      .at(-1)?.run;
  });

  let rounds = $state<RoundsAnswer | undefined>(undefined);
  $effect(() => {
    const at = run;
    rounds = undefined;
    if (at === undefined) return;
    return u.conn.asking.ask({ rounds: { run: at } }).subscribe((held) => {
      rounds = held !== undefined && "rounds" in held ? held.rounds : undefined;
    });
  });

  let second = $state<number | null>(null);
  $effect(() => {
    const at = room;
    second = null;
    if (at === null) return;
    return u.conn.asking.ask({ config: { addr: at } }).subscribe((held) => {
      second = held !== undefined && "config" in held ? held.config.second.percent : null;
    });
  });

  const context = $derived(
    contextOf(
      rounds?.turns ?? [],
      $endpoints !== undefined && "endpoints" in $endpoints ? $endpoints.endpoints.endpoints : [],
      second,
    ),
  );
  const arc = $derived(context === null ? null : remainingArc(context));

  // The one line the ring says, and the whole of what a screen reader is
  // told: used against the window, the share, and the reminders the page
  // knows about.
  const reading = $derived.by((): string => {
    if (context === null) return "";
    const parts = [
      fill(say($lang, "ring_used"), { used: kilo(context.used), window: kilo(context.window) }),
      fill(say($lang, "ring_share"), { n: String(usedPercent(context)) }),
    ];
    if (context.first !== null) parts.push(fill(say($lang, "ring_first"), { n: String(context.first) }));
    if (context.second !== null) parts.push(fill(say($lang, "ring_second"), { n: String(context.second) }));
    return parts.join(" · ");
  });
</script>

{#if context === null || arc === null}
  <span class="grid size-key shrink-0 place-items-center">{@render coin()}</span>
{:else}
  <Tip text={reading}>
    {#snippet children(hint)}
      <span class="relative grid size-key shrink-0 place-items-center">
        <!-- svelte-ignore a11y_no_noninteractive_tabindex (the meter takes focus so a keyboard reaches the reading its tip spells out, as a pointer does by hovering) -->
        <span
          class="absolute inset-0 rounded-pill"
          role="meter"
          tabindex="0"
          aria-label={say($lang, "ring_name")}
          aria-valuemin={0}
          aria-valuemax={context.window}
          aria-valuenow={Math.min(context.used, context.window)}
          aria-valuetext={reading}
          aria-describedby={hint}
        ></span>
        <svg class="pointer-events-none absolute inset-0 -rotate-90" viewBox="0 0 40 40" aria-hidden="true">
          <circle
            class="ring-left fill-none stroke-text"
            cx="20"
            cy="20"
            r="18"
            stroke-width="1"
            pathLength="100"
            stroke-dasharray={arc.dasharray}
            stroke-dashoffset={arc.dashoffset}
          />
        </svg>
        {#if context.first !== null}
          <i class="rung pointer-events-none bg-reminder-first" style:--at={context.first} aria-hidden="true"></i>
        {/if}
        {#if context.second !== null}
          <i class="rung pointer-events-none bg-reminder-second" style:--at={context.second} aria-hidden="true"></i>
        {/if}
        {@render coin()}
      </span>
    {/snippet}
  </Tip>
{/if}
