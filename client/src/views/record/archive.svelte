<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The archive as the buildings filed it, searched across every
// building at once: one row per hit, and the building is the link.
-->

<script lang="ts">
  import { toFragment } from "../../core/route";
  import { readAnswer } from "../../core/answered";
  import { say } from "../../core/lang";
  import { ui } from "../../ui";
  import type { Query } from "../../wire";
  import Unanswered from "../parts/unanswered.svelte";

  const u = ui();
  const lang = u.lang;

  let needle = $state("");

  const question = $derived<Query>({ archive_search: { needle } });
  const search = $derived(u.conn.asking.ask(question));
  const read = $derived(readAnswer($search, (held) => ("archive" in held ? held.archive.hits : undefined)));
  const hits = $derived(read.kind === "held" ? read.value : undefined);
</script>

<div>
  <input
    class="mb-base w-full rounded-control border border-edge-input bg-raised px-base py-snug text-body placeholder:text-text-disabled"
    placeholder={say($lang, "rec_search")}
    bind:value={needle}
  />
  {#if read.kind === "unavailable"}
    <Unanswered query={read.query} asked={question} />
  {:else if hits === undefined}
    <p class="text-text-disabled">…</p>
  {:else if hits.length === 0}
    <p class="text-text-faint">{say($lang, "rec_nothing")}</p>
  {:else}
    <ul class="text-note">
      {#each hits as hit (hit)}
        <li class="settled-row flex gap-base border-b border-edge py-snug">
          <span class="w-figure shrink-0 text-text-disabled">{hit.day}</span>
          <span class="w-figure shrink-0 text-text-faint">{hit.kind}</span>
          <a
            href={toFragment({ kind: "building", address: hit.building })}
            class="block max-w-[24ch] shrink-0 truncate text-text-quiet"
          >
            {hit.building}
          </a>
          <span class="flex-1 truncate text-text">{hit.subject}</span>
        </li>
      {/each}
    </ul>
  {/if}
</div>
