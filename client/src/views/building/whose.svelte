<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // One commit asked for by its oid (`Query::Commit`, client-SPEC 4-24,
  // 4-50): the question the command line spells `whose`. The city
  // answers it from the Ledger, so a commit it never wrote is
  // `Unavailable`, and the page says that rather than "nothing changed".
  import { readAnswer } from "../../core/answered";
  import { fill, say } from "../../core/lang";
  import { ui } from "../../ui";
  import type { GitOid, Query } from "../../wire";
  import { shortOid } from "../changes";
  import CommitFacts from "./commit_facts.svelte";
  import CommitLine from "./commit_line.svelte";

  interface Props {
    readonly oid: GitOid;
  }

  const { oid }: Props = $props();

  const u = ui();
  const lang = u.lang;

  const question = $derived<Query>({ commit: { oid } });
  const asked = $derived(u.conn.asking.ask(question));
  const read = $derived(readAnswer($asked, (held) => ("commit" in held ? held.commit : undefined)));
</script>

{#if read.kind === "held"}
  <div class="flex min-w-0 flex-col gap-snug">
    <p class="grid h-control grid-cols-[8ch_minmax(0,1fr)_auto_auto] items-center gap-x-base text-note">
      <CommitLine commit={read.value} />
    </p>
    <CommitFacts commit={read.value} />
  </div>
{:else if read.kind === "unavailable"}
  <p class="text-note text-text-faint">{fill(say($lang, "whose_unknown"), { oid: shortOid(oid) })}</p>
{:else}
  <p class="text-note text-text-faint">…</p>
{/if}
