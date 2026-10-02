<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  // The context ring round the coin key (docs/frontend-method.md §7J): where the eye
  // already is, without taking a place of its own. A whole one-pixel
  // ring is an empty window; the used part is eaten away clockwise from
  // twelve o'clock, and two checkpoints mark where the reminders sound.
  // The ring stands still while the key turns over inside it.
  //
  // It asks three questions the page already asks elsewhere, and
  // `asking` merges them by content: the session's rounds, the
  // endpoints with their models' windows, and this room's settled
  // config, which states both reminders. What it draws from them is
  // `gauge.ts`'s.
  import type { Snippet } from "svelte";

  import { QUERIES } from "../../core/asking";
  import { fill, say } from "../../core/lang";
  import { kilo } from "../../core/time";
  import { ui } from "../../ui";
  import type { Address, RoundsAnswer } from "../../wire";
  import Tip from "../parts/tip.svelte";
  import { contextOf, remainingArc, remindersOf, remindersSaid, sessionRunOf, usedPercent } from "./gauge";
  import type { Reminders } from "./gauge";

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

  const run = $derived(room === null ? undefined : sessionRunOf($belief, room));

  let rounds = $state<RoundsAnswer | undefined>(undefined);
  $effect(() => {
    const at = run;
    rounds = undefined;
    if (at === undefined) return;
    return u.conn.asking.ask({ rounds: { run: at } }).subscribe((held) => {
      rounds = held !== undefined && "rounds" in held ? held.rounds : undefined;
    });
  });

  let reminders = $state<Reminders>(remindersOf(undefined));
  $effect(() => {
    const at = room;
    reminders = remindersOf(undefined);
    if (at === null) return;
    return u.conn.asking.ask({ config: { addr: at } }).subscribe((held) => {
      reminders = remindersOf(held);
    });
  });

  const context = $derived(
    contextOf(
      rounds?.turns ?? [],
      $endpoints !== undefined && "endpoints" in $endpoints ? $endpoints.endpoints.endpoints : [],
      reminders,
    ),
  );
  const arc = $derived(context === null ? null : remainingArc(context));

  // The one line the ring says, and the whole of what a screen reader is
  // told: used against the window, the share, and the reminders the page
  // knows about.
  const reading = $derived.by((): string => {
    if (context === null) return "";
    return [
      fill(say($lang, "ring_used"), { used: kilo(context.used), window: kilo(context.window) }),
      fill(say($lang, "ring_share"), { n: String(usedPercent(context)) }),
      ...remindersSaid($lang, context),
    ].join(" · ");
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
