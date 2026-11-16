<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  import { readAnswer } from "../../core/answered";
  import { QUERIES } from "../../core/asking";
  import { heldWithin } from "../../core/belief/rooms";
  import { ui } from "../../ui";
  import type { Address } from "../../wire";
  import Skyline, { MIN_WIDTH } from "../city/skyline.svelte";
  import Unanswered from "../parts/unanswered.svelte";

  const u = ui();
  const belief = u.conn.belief;
  const city = u.conn.asking.ask(QUERIES.city);
  const read = $derived(readAnswer($city, (held) => ("city" in held ? held.city : undefined)));

  function centred(box: HTMLDivElement): void {
    box.scrollLeft = (box.scrollWidth - box.clientWidth) / 2;
  }

  function enter(building: Address): void {
    const newest = heldWithin($belief, building)
      .filter((run) => run.addr !== null)
      .sort((a, b) => (a.started ?? 0) - (b.started ?? 0))
      .at(-1)?.addr;
    u.go({ kind: "talk", address: newest ?? building });
  }
</script>

{#if read.kind === "unavailable"}
  <Unanswered query={read.query} asked={QUERIES.city} />
{:else if read.kind === "held"}
  <div class="min-h-0 overflow-x-auto" {@attach centred}>
    <div style:min-width="{MIN_WIDTH}px">
      <Skyline city={read.value} picked={null} onPick={enter} />
    </div>
  </div>
{/if}
