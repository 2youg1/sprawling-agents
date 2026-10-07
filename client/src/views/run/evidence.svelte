<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // The evidence lens: what the run left to be checked - a screenshot,
  // the word that it finished - one row each, in the order the ledger
  // wrote them. The question is asked while this lens is mounted and no
  // longer: a watched answer is refreshed when stale, and nobody is
  // looking at this one between visits (`parts/tabs` mounts a panel
  // only while its lens is current).

  import { readAnswer } from "../../core/answered";
  import type { Key } from "../../core/lang";
  import { say } from "../../core/lang";
  import { ui } from "../../ui";
  import type { EvidenceKind, Query, RunId } from "../../wire";
  import EmptyState from "../parts/empty.svelte";
  import Unanswered from "../parts/unanswered.svelte";

  interface Props {
    readonly run: RunId;
  }

  const { run }: Props = $props();

  const EVIDENCE_WORD: Record<EvidenceKind, Key> = {
    screenshot: "evidence_screenshot",
    finished: "evidence_finished",
  };

  const u = ui();
  const lang = u.lang;

  const question = $derived<Query>({ evidence: { run } });
  const asked = $derived(u.conn.asking.ask(question));
  const read = $derived(readAnswer($asked, (held) => ("evidence" in held ? held.evidence.items : undefined)));
  const items = $derived(read.kind === "held" ? read.value : undefined);
</script>

{#if read.kind === "unavailable"}
  <Unanswered query={read.query} asked={question} />
{:else if items === undefined}
  <p class="text-text-faint">…</p>
{:else if items.length > 0}
  <ul class="text-note">
    {#each items as item (item.at)}
      <li class="flex items-center gap-base border-b border-edge py-snug">
        <span class="w-figure shrink-0 text-text-faint">{say($lang, EVIDENCE_WORD[item.kind])}</span>
        <span class="flex-1 truncate font-mono text-text-quiet">{item.locator}</span>
        {#if item.picture !== null && item.picture !== undefined}
          <span class="figure text-text-faint">{item.picture.width}×{item.picture.height} {item.picture.media_type}</span>
        {/if}
        <span class="figure text-text-faint">#{item.at}</span>
      </li>
    {/each}
  </ul>
{:else}
  <EmptyState missing="run_no_evidence" />
{/if}
