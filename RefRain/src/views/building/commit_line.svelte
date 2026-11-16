<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  
  
  import { say } from "../../core/lang";
  import { roomOf } from "../../core/route";
  import { ui } from "../../ui";
  import { isoInstant, moment } from "../../core/time";
  import type { CommitAnswer } from "../../wire";
  import { shortOid } from "../changes";

  interface Props {
    readonly commit: CommitAnswer;
  }

  const { commit }: Props = $props();

  const lang = ui().lang;

  
  const subject = $derived(commit.message?.split("\n", 1)[0]?.trim() ?? "");
  
  
</script>

<span class="figure text-text-quiet">{shortOid(commit.oid)}</span>
{#if subject === ""}
  <span class="truncate text-text-faint">{say($lang, "commits_no_message")}</span>
{:else}
  <span class="truncate text-text">{subject}</span>
{/if}
<span class="truncate font-mono text-text-faint narrow:hidden">{roomOf(commit.actor)}</span>
<time class="figure text-text-faint narrow:hidden" datetime={isoInstant(commit.at)}
  >{moment(commit.at)}</time
>
