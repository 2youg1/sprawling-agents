<script lang="ts" module>
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What this city has decided is worth keeping: one line per asset a run
// filed, with when it was filed, what kind of thing it is, the building
// it belongs to, and what it is about.
//
// The registry is the one place an external event that outlived its run
// can be found, and until this screen existed `Query::RegistryView` had
// no reader at all - a question the city answers and nobody asks is a
// defect of the same family as an answer nobody draws.

export { default as RegistryTable } from "./registry/table.svelte";
</script>

<script lang="ts">
  import { QUERIES } from "../core/asking";
  import { say } from "../core/lang";
  import { ui } from "../ui";
  import type { RegistryAnswer } from "../wire";
  import Table from "./registry/table.svelte";

  const u = ui();
  const lang = u.lang;
  const asked = u.conn.asking.ask(QUERIES.registry);

  const answer = $derived.by((): RegistryAnswer | undefined => {
    const held = $asked;
    return held !== undefined && "registry" in held ? held.registry : undefined;
  });
</script>

<!-- The screen: the one question, and the table that draws its answer.
A table is not capped by content kind (client-SPEC 4-33) - it grows with
its container and scrolls sideways rather than break a value. -->
<div class="w-full px-pane py-wide">
  <h1 class="mb-wide text-title font-title" tabindex="-1">{say($lang, "nav_registry")}</h1>
  {#if answer === undefined}
    <p class="text-text-faint">…</p>
  {:else}
    <Table assets={answer.assets} />
  {/if}
</div>
