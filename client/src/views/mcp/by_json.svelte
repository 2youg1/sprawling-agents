<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  // `claude mcp add-json`, as a form: paste the block another tool wrote
  // and read, before anything is sent, which of its servers this city
  // can be told about and why the rest cannot.
  //
  // The block is shown row by row rather than accepted whole: a
  // settings file usually holds several servers, and one of them a
  // person has to fix must not silently take the other three with it.

  import type { Draft, Encoded, Intake } from "./draft";

  export interface ByJsonProps {
    readonly intake: Intake;
  }

  // One row of the read block: what it spelled, and whether that draft
  // can travel as spelled.
  interface Read {
    readonly draft: Draft;
    readonly encoded: Encoded;
  }
</script>

<script lang="ts">
  import { SPELLED, WHY, encode } from "./draft";
  import { readFragment } from "./fragment";
  import { say } from "../../core/lang";
  import { ui } from "../../ui";
  import Badge from "../parts/badge.svelte";
  import Button from "../parts/button.svelte";

  const { intake }: ByJsonProps = $props();

  const { lang } = ui();

  let text = $state("");

  const fragment = $derived(readFragment(text));
  const rows = $derived.by((): readonly Read[] => {
    const read = fragment;
    if (read.kind === "unreadable") return [];
    return read.drafts.map((draft) => ({ draft, encoded: encode(draft, intake.taken()) }));
  });
  const ready = $derived(rows.filter((row) => row.encoded.kind === "ready"));
  const reason = $derived.by((): string | null => {
    const scope = intake.why();
    if (scope !== null) return scope;
    return ready.length === 0 ? say($lang, "mcp_json_empty") : null;
  });

  function send(): void {
    let every = true;
    for (const row of rows) {
      if (row.encoded.kind === "ready") {
        every = intake.offer(row.encoded.server) && every;
      } else {
        every = false;
      }
    }
    if (every) {
      text = "";
    }
  }
</script>

<div class="flex flex-col gap-base">
  <textarea
    class="min-h-output w-full min-w-0 rounded-control border border-edge-panel bg-raised px-base py-snug font-mono text-note text-text placeholder:text-text-disabled"
    rows={8}
    aria-label={say($lang, "mcp_door_json")}
    placeholder={say($lang, "mcp_json_placeholder")}
    value={text}
    oninput={(event) => {
      text = event.currentTarget.value;
    }}
  ></textarea>
  {#if text.trim() !== "" && fragment.kind === "unreadable"}
    <p class="text-note text-alert" role="alert">
      {say($lang, "mcp_json_unreadable")}
    </p>
  {/if}
  <ul class="flex flex-col gap-tight">
    {#each rows as row (row.draft.label + row.draft.url + row.draft.command)}
      <li class="flex min-w-0 items-center gap-base rounded-card bg-chrome px-base py-snug text-note">
        <span class="w-figure shrink-0 truncate font-mono text-text">{row.draft.label}</span>
        <Badge text={say($lang, SPELLED[row.draft.transport])} />
        {#if row.encoded.kind === "blocked"}
          <span class="min-w-0 flex-1 text-text-faint">{say($lang, WHY[row.encoded.blocker])}</span>
        {:else}
          <span class="min-w-0 flex-1 truncate font-mono text-text-faint">
            {row.draft.url === "" ? row.draft.command : row.draft.url}
          </span>
        {/if}
      </li>
    {/each}
  </ul>
  {#if reason === null}
    <Button label={say($lang, "mcp_add_all")} tone="primary" onPress={send} />
  {:else}
    <div class="flex min-w-0 items-center gap-base">
      <Button label={say($lang, "mcp_add_all")} why={reason} />
      <span class="min-w-0 text-note text-text-faint">{reason}</span>
    </div>
  {/if}
</div>
