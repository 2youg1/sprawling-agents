<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  // The world layer's third pane follows the workspace (refrain roadmap
  // Q10, client/Spec.lean §7K): it draws the place the conversation is in. Its
  // first reading is always that place's commits as a graph; its second
  // is the city itself from the Mayor's room, whose place is the city,
  // and the building's files from a room inside a building.
  //
  // One pane with two readings rather than a fourth column, because the
  // workbench's three columns are the person's to order and size, and
  // the city drawing and the file tree are two answers to the one
  // question this pane asks - where is this work happening.
  import type { Snippet } from "svelte";

  import { say } from "../../core/lang";
  import { MAYOR } from "../../core/route";
  import { ui } from "../../ui";
  import type { Address } from "../../wire";
  import Tabs from "../parts/tabs.svelte";
  import City from "./city.svelte";
  import Commits from "./commits.svelte";
  import Files from "./files.svelte";

  interface Props {
    readonly here: Address;
    readonly head: Snippet;
  }

  const { here, head }: Props = $props();
  const { lang } = ui();

  let reading = $state<"commits" | "place">("commits");

  const lenses = $derived([
    { id: "commits", label: say($lang, "world_commits") },
    { id: "place", label: say($lang, here === MAYOR ? "world_city" : "world_files") },
  ]);
</script>

<!-- The tab panels are this column's children, and the one shown takes
the height left under the tab strip. -->
<section
  class="flex min-h-0 flex-1 flex-col overflow-hidden [&>[role=tabpanel]]:flex [&>[role=tabpanel]]:min-h-0 [&>[role=tabpanel]]:flex-1 [&>[role=tabpanel]]:flex-col [&>[role=tabpanel]]:pt-snug"
  aria-label={say($lang, "world_place")}
>
  {@render head()}
  <Tabs
    label={say($lang, "world_place")}
    {lenses}
    current={reading}
    onPick={(id) => {
      reading = id === "place" ? "place" : "commits";
    }}
  >
    {#snippet panel(lens)}
      {#if lens.id === "commits"}
        <Commits {here} />
      {:else if here === MAYOR}
        <City />
      {:else}
        <Files {here} />
      {/if}
    {/snippet}
  </Tabs>
</section>
