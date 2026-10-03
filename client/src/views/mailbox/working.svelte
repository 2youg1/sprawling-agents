<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // The second section of the mailbox: what is going now, one row per
  // room - its newest run that has not frozen, read from `belief.live`,
  // the one answer to which runs are working (client/Spec.lean §7). A row
  // names the room, the phase in its mark and word, how long the run has
  // gone, and its task; it is a link to that room's conversation. The
  // newest room first, the order a person who just dispatched looks in.
  // A run waiting for a reply says which room it waits on and how long
  // is left, on the same clock as its age (client/Spec.lean D88).
  import type { RunBelief } from "../../core/belief";
  import { toFragment } from "../../core/route";
  import { lasted } from "../../core/time";
  import { ui } from "../../ui";
  import type { Address } from "../../wire";
  import Glyph from "../parts/glyph.svelte";
  import type { Weight } from "../parts/glyph";
  import { phaseOf } from "../runs/lineage";
  import { PHASE_MARK, phaseSaid } from "../runs/phase";
  import Section from "./section.svelte";
  import { ticker } from "../talk/timing";

  interface Props {
    // Following a row leaves the mailbox for the room.
    readonly onLeave: () => void;
  }

  const { onLeave }: Props = $props();

  // The tier a phase's mark is painted in, as the runs board paints it.
  const INK: Readonly<Record<Weight, string>> = { quiet: "text-text-quiet", live: "text-accent", alert: "text-alert" };

  const u = ui();
  const { lang } = u;
  const belief = u.conn.belief;

  // `live` runs oldest to newest, so the last run seen in a room is its
  // newest; a run with no room yet has nowhere to link to.
  const rows = $derived.by(() => {
    const newest: Record<string, { readonly room: Address; readonly run: RunBelief }> = {};
    for (const run of $belief.live) {
      if (run.addr !== null) newest[run.addr] = { room: run.addr, run };
    }
    return Object.values(newest).sort((a, b) => (b.run.started ?? 0) - (a.run.started ?? 0));
  });

  // How long each row's run has gone, on the page's one clock: it moves
  // only while a row reads it and the page is seen, and every reading is
  // recomputed from the run's own start (client/Spec.lean §4-59).
  const tick = ticker(u.now);
</script>

<Section title="mailbox_working" empty="mailbox_working_none" count={rows.length}>
  <ul>
    {#each rows as { room, run } (run.run)}
      {@const phase = phaseOf(run.doing)}
      {@const mark = PHASE_MARK[phase]}
      <li>
        <a
          href={toFragment({ kind: "talk", address: room })}
          class="grid grid-cols-[var(--spacing-glyph-sm)_minmax(0,1fr)_auto_auto] items-center gap-x-snug rounded-card px-snug py-tight hover:wash focus-visible:wash"
          data-entry
          onclick={onLeave}
        >
          <Glyph name={mark.glyph} size="sm" class={INK[mark.weight]} />
          <span class="min-w-0 truncate font-label">{room}</span>
          <span class="figure text-note text-text-faint">
            {phaseSaid($lang, run.doing, $tick)} · {run.started === null ? "" : lasted($tick - run.started)}
          </span>
          <kbd class="entry-n" aria-hidden="true"></kbd>
          {#if run.task !== null}
            <span class="col-start-2 col-end-5 truncate text-note text-text-quiet">{run.task}</span>
          {/if}
        </a>
      </li>
    {/each}
  </ul>
</Section>
