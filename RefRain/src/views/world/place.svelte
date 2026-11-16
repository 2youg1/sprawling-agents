<script lang="ts">
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
