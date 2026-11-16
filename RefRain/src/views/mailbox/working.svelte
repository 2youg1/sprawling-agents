<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  import type { RunBelief } from "../../core/belief";
  import { toFragment } from "../../core/route";
  import { ui } from "../../ui";
  import type { Address } from "../../wire";
  import { ticker } from "../talk/timing";
  import Section from "./section.svelte";
  import { rowOf } from "./working_row";
  import Row from "./working_row.look.svelte";

  interface Props {
    readonly onLeave: () => void;
  }

  const { onLeave }: Props = $props();

  const u = ui();
  const { lang } = u;
  const belief = u.conn.belief;

  const rows = $derived.by(() => {
    const newest: Record<string, { readonly room: Address; readonly run: RunBelief }> = {};
    for (const run of $belief.live) {
      if (run.addr !== null) newest[run.addr] = { room: run.addr, run };
    }
    return Object.values(newest).sort((a, b) => (b.run.started ?? 0) - (a.run.started ?? 0));
  });

  const tick = ticker(u.now);
</script>

<Section title="mailbox_working" empty="mailbox_working_none" count={rows.length}>
  <ul>
    {#each rows as { room, run } (run.run)}
      <Row
        {...rowOf(run, room, $lang, $tick, {
          href: toFragment({ kind: "talk", address: room }),
          onclick: onLeave,
        })}
      />
    {/each}
  </ul>
</Section>
