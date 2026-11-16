<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // The seat of the recycle bin: it asks the city, lends the decision
  // (`./bin`) the person's language and the restore it sends, and draws
  // whatever `./bin.look.svelte` is.

  import { QUERIES } from "../../core/asking";
  import { restoreDiscard } from "../../core/commands";
  import { ui } from "../../ui";
  import EmptyState from "../parts/empty.svelte";
  import { lookOf } from "./bin";
  import Look from "./bin.look.svelte";

  const u = ui();
  const lang = u.lang;
  const held = u.conn.asking.ask(QUERIES.discards);

  const rows = $derived.by(() => {
    const answer = $held;
    return answer !== undefined && "discards" in answer ? answer.discards.rows : undefined;
  });
</script>

{#if rows === undefined}
  <p class="text-text-faint">…</p>
{:else if rows.length === 0}
  <EmptyState missing="bin_nothing" seat="region" />
{:else}
  <Look
    {...lookOf(rows, $lang, (restoration) => {
      u.send(restoreDiscard(restoration));
    })}
  />
{/if}
