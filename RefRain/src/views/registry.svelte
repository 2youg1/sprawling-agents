<script lang="ts" module>
  // This Source Code Form is subject to the terms of the Mozilla Public
  // License, v. 2.0. If a copy of the MPL was not distributed with this
  // file, You can obtain one at https://mozilla.org/MPL/2.0/.
  // Copyright (c) 2026 2youg1 and the sprawling contributors

  export { default as RegistryTable } from "./registry/table.svelte";
</script>

<script lang="ts">
  import { QUERIES } from "../core/asking";
  import { readAnswer } from "../core/answered";
  import { fill, say } from "../core/lang";
  import { ui } from "../ui";
  import Page from "./parts/page.svelte";
  import Unanswered from "./parts/unanswered.svelte";
  import Table from "./registry/table.svelte";

  interface Props {
    readonly rank?: "page" | "section" | undefined;
  }

  const { rank = "page" }: Props = $props();

  const u = ui();
  const lang = u.lang;
  const asked = u.conn.asking.ask(QUERIES.registry);

  const read = $derived(
    readAnswer($asked, (held) => ("registry" in held ? held.registry : undefined)),
  );
  const answer = $derived(read.kind === "held" ? read.value : undefined);
</script>

{#snippet count()}
  {#if answer !== undefined && answer.assets.length > 0}
    <span class="figure text-note text-text-quiet"
      >{fill(say($lang, "registry_count"), { n: String(answer.assets.length) })}</span
    >
  {/if}
{/snippet}

<Page title={say($lang, "nav_registry")} {rank} note={say($lang, "registry_note")} aside={count}>
  {#if read.kind === "unavailable"}
    <Unanswered query={read.query} asked={QUERIES.registry} />
  {:else if answer === undefined}
    <p class="text-text-faint">…</p>
  {:else}
    <Table assets={answer.assets} />
  {/if}
</Page>
