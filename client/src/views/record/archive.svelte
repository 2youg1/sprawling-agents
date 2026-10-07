<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The archive as the buildings filed it, searched across every
// building at once: one row per hit, and the building is the link. This
// seat asks the city and places the search box over the answer; the
// hits are drawn by whatever `./archive.look.svelte` is.
-->

<script lang="ts">
  import { readAnswer } from "../../core/answered";
  import { say } from "../../core/lang";
  import { ui } from "../../ui";
  import type { Query } from "../../wire";
  import EmptyState from "../parts/empty.svelte";
  import Field from "../parts/field.svelte";
  import Unanswered from "../parts/unanswered.svelte";
  import { lookOf } from "./archive";
  import Look from "./archive.look.svelte";

  const u = ui();
  const lang = u.lang;

  let needle = $state("");

  const question = $derived<Query>({ archive_search: { needle } });
  const search = $derived(u.conn.asking.ask(question));
  const read = $derived(readAnswer($search, (held) => ("archive" in held ? held.archive.hits : undefined)));
  const hits = $derived(read.kind === "held" ? read.value : undefined);
</script>

<div class="flex flex-col gap-base">
  <!-- The box is named by the word it used to show only as a
       placeholder, which a screen reader does not take for a name. -->
  <div class="max-w-talk">
    <Field
      label={say($lang, "rec_search")}
      labelling="hidden"
      placeholder={say($lang, "rec_search")}
      value={needle}
      onInput={(value) => {
        needle = value;
      }}
    />
  </div>
  {#if read.kind === "unavailable"}
    <Unanswered query={read.query} asked={question} />
  {:else if hits === undefined}
    <p class="text-text-faint">…</p>
  {:else if hits.length === 0}
    <EmptyState missing="rec_archive_nothing" seat="region" />
  {:else}
    <Look {...lookOf(hits)} />
  {/if}
</div>
