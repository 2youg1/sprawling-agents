<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  // The seat of the city's row of figures: it asks the city for its
  // metrics and its spend, and hands `./bar.ts`'s value to whatever
  // `./bar.look.svelte` is. Before either answer arrives the bar is not
  // drawn at all.
  import { QUERIES } from "../../core/asking";
  import { ui } from "../../ui";
  import { barLookOf } from "./bar";
  import Look from "./bar.look.svelte";

  const u = ui();
  const lang = u.lang;
  const metrics = u.conn.asking.ask(QUERIES.metrics);
  const cost = u.conn.asking.ask(QUERIES.cost);

  const look = $derived(
    barLookOf(
      $metrics !== undefined && "metrics" in $metrics ? $metrics.metrics : null,
      $cost !== undefined && "cost" in $cost ? $cost.cost.total : null,
      $lang,
    ),
  );
</script>

{#if look !== null}
  <Look {...look} />
{/if}
