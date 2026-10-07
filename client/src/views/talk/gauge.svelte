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
  // `gauge.ts`'s, and how it is drawn is `gauge.look.svelte`'s: this seat
  // only asks and hands the answer on.
  import type { Snippet } from "svelte";

  import { QUERIES } from "../../core/asking";
  import { ui } from "../../ui";
  import type { Address, RoundsAnswer } from "../../wire";
  import Look from "./gauge.look.svelte";
  import { contextOf, remindersOf, ringOf, sessionRunOf } from "./gauge";
  import type { Reminders } from "./gauge";

  interface Props {
    // The room the composer speaks to; its newest session is the one the
    // ring measures.
    readonly room: Address | null;
    readonly children: Snippet;
  }

  const { room, children }: Props = $props();

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

  const ring = $derived(
    ringOf(
      $lang,
      contextOf(
        rounds?.turns ?? [],
        $endpoints !== undefined && "endpoints" in $endpoints ? $endpoints.endpoints.endpoints : [],
        reminders,
      ),
    ),
  );
</script>

<Look {ring} {children} />
