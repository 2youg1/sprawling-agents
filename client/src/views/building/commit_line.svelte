<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // One commit as one line (client/Spec.lean §4-50, UC2): its short oid, the
  // first line of its message, the room that wrote it and the moment,
  // in ISO form to the second. The cells sit on the grid of whatever
  // draws the line - a row of the commit list, or the head of a commit
  // asked for by its oid - so the columns of every line on the page
  // are the same columns.
  import { say } from "../../core/lang";
  import { roomOf } from "../../core/route";
  import { ui } from "../../ui";
  import { isoInstant } from "../../core/time";
  import type { CommitAnswer } from "../../wire";
  import { shortOid } from "../changes";

  interface Props {
    readonly commit: CommitAnswer;
  }

  const { commit }: Props = $props();

  const lang = ui().lang;

  // A commit written before the city carried messages has none; the
  // line says so instead of leaving a gap that reads as a blank message.
  const subject = $derived(commit.message?.split("\n", 1)[0]?.trim() ?? "");
  // To the second: the milliseconds are the ledger's, and a commit row
  // is read for the moment, not for the order within one second.
  const iso = $derived(isoInstant(commit.at));
</script>

<span class="figure text-text-quiet">{shortOid(commit.oid)}</span>
{#if subject === ""}
  <span class="truncate text-text-faint">{say($lang, "commits_no_message")}</span>
{:else}
  <span class="truncate text-text">{subject}</span>
{/if}
<span class="truncate font-mono text-text-faint narrow:hidden">{roomOf(commit.actor)}</span>
<time class="figure text-text-faint narrow:hidden" datetime={iso}>{`${iso.slice(0, 19)}Z`}</time>
