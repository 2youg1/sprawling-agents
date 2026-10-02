<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  // What this city can do, and where a person puts one more of it: the
  // buildings on the left, that building's shelves in the middle, and
  // the skill a person opened on the right.
  //
  // **A shelf is read here and written on disk.** The city's library
  // sits under the reserved prefix, which no write domain reaches - a
  // resident may read the stock and may not restock it (`city::Library`)
  // - so the wire has a question for the shelves and no frame that puts
  // a document on one. The two folder paths are therefore part of the
  // screen rather than a footnote: they are how a person adds a skill,
  // and the list beside them is how they see it arrive.
  //
  // **Both the list and the reader are the building page's.** A second
  // drawing of a shelf row, or of a document, would be a second answer
  // to "what does this city have" the first time one of them was
  // corrected.
</script>

<script lang="ts">
  import { say } from "../../core/lang";
  import { ui } from "../../ui";
  import { Address } from "../../wire";
  import FileView from "../building/file.svelte";
  import Skills from "../building/skills.svelte";
  import EmptyState from "../parts/empty.svelte";
  import { BuildingColumn, HALL, useBuildings } from "../shared/buildings";
  import Shelves from "./shelves.svelte";

  const { lang } = ui();
  const buildings = useBuildings();

  let chosen = $state<Address>(HALL);
  // The skill whose document is open, or nothing when the person has
  // opened none. Cleared when the building changes, because a document
  // from one building's shelf under another building's list is a claim
  // about where that skill sits.
  let reading = $state<Address | null>(null);
</script>

<div class="flex min-w-0 flex-col gap-wide">
  <Shelves />
  <!-- The two columns are grid columns rather than flex ones because a
       column of prose has a width below which it stops being prose: the
       flex row let the reading pane shrink toward nothing at a narrow
       window, and the empty state under it wrapped one character per
       line (client/Spec.lean §4-33's third rule). On a wide page the
       building list keeps its own width and the reading pane takes
       the rest, as the MCP page draws the same pair. -->
  <div class="grid items-start gap-wide @wide/page:grid-cols-[var(--spacing-tree)_minmax(0,1fr)]">
    <BuildingColumn
      label={say($lang, "mcp_building")}
      buildings={$buildings}
      {chosen}
      hall={say($lang, "city_hall")}
      onPick={(addr: Address) => {
        chosen = addr;
        reading = null;
      }}
    />
    <div class="flex min-w-0 flex-col gap-wide">
      <Skills
        building={chosen}
        onPick={(picked: { at: string }) => {
          // The pick crosses this seam typed as a plain string -
          // `building/tree.svelte`'s `Picked` - but its origin is a
          // wire-typed shelf address (`placeOf` reads `shelf.building` /
          // `shelf.library`), so the constructor that brands a string
          // this file already believes is the honest reading here.
          reading = Address.make(picked.at);
        }}
      />
      {#if reading !== null}
        <FileView at={reading} root={chosen} />
      {:else}
        <EmptyState missing="skills_unopened" seat="region" />
      {/if}
    </div>
  </div>
</div>
