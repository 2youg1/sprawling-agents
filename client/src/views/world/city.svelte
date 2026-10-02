<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  // The city, as the world layer draws it beside the Mayor's room
  // (refrain roadmap Q10, 3-9): the city page's skyline at the size the
  // city page draws it, scrolled sideways to City Hall at its centre. A
  // pane is narrower than the drawing, and the drawing scaled down to a
  // pane's width puts every tower's name under seven pixels, so the
  // pane scrolls rather than shrinks it. Picking a tower moves the
  // workspace there - to the newest room
  // of that building that ran anything, or to the building itself, where
  // the conversation opens a room of its own.
  import { readAnswer } from "../../core/answered";
  import { QUERIES } from "../../core/asking";
  import { heldWithin } from "../../core/belief/rooms";
  import { ui } from "../../ui";
  import type { Address } from "../../wire";
  import Skyline from "../city/skyline.svelte";
  import Unanswered from "../parts/unanswered.svelte";

  const u = ui();
  const belief = u.conn.belief;
  const city = u.conn.asking.ask(QUERIES.city);
  const read = $derived(readAnswer($city, (held) => ("city" in held ? held.city : undefined)));

  // The skyline's own narrowest width, copied from `city/skyline.svelte`'s
  // `MIN_WIDTH` until that file exports it.
  const DRAWN = 880;

  // City Hall stands at the centre of the avenue, so the pane opens there.
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
    <div style:min-width="{DRAWN}px">
      <Skyline city={read.value} picked={null} onPick={enter} />
    </div>
  </div>
{/if}
