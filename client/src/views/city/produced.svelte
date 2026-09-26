<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  // What a finished run of the results city produced, at the end of its
  // row. The city answer carries no produced figures, so the row asks
  // the run's rounds for the two trees the room's line measures between
  // and hands them to that same line; only the rows drawn ask, which the
  // city's `FIRST` cut keeps to a handful.
  import Produced from "../talk/produced.svelte";
  import { lastFenceIn } from "../talk/trace";
  import { ui } from "../../ui";
  import type { RunId } from "../../wire";

  interface Props {
    readonly run: RunId;
  }

  const { run }: Props = $props();
  const u = ui();

  const asked = $derived(u.conn.asking.ask({ rounds: { run } }));
  const span = $derived.by(() => {
    const held = $asked;
    if (held === undefined || !("rounds" in held)) return null;
    const base = held.rounds.opened_at ?? null;
    const head = lastFenceIn(held.rounds.turns);
    return base === null ? null : { base, head: head === base ? null : head };
  });
</script>

{#if span !== null}
  <Produced base={span.base} head={span.head} place="row" />
{/if}
