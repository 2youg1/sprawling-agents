<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->
<script lang="ts">
  // What was answered on the person's behalf, under the control that
  // delegated it. A delegation whose use nobody can see is a delegation
  // nobody can judge, so the record sits where the choice is made.
  // Newest first: the question a person brings here is "what did it
  // just decide for me".

  import { QUERIES } from "../../core/asking";
  import { say } from "../../core/lang";
  import { clock } from "../../core/time";
  import { ui } from "../../ui";
  import type { Decision } from "../../wire";

  interface Props {
    // The record a caller hands in place of a city; absent, this
    // section asks the city its own question.
    readonly decided?: readonly Decision[] | undefined;
  }

  const { decided }: Props = $props();

  const u = ui();
  const lang = u.lang;
  const governance = u.conn.asking.ask(QUERIES.governance);

  const shown = $derived.by((): readonly Decision[] | undefined => {
    if (decided !== undefined) return decided;
    const held = $governance;
    return held !== undefined && "governance" in held ? held.governance.decided : undefined;
  });
  const newest = $derived(shown === undefined ? undefined : [...shown].sort((a, b) => b.at - a.at));
</script>

<div class="flex flex-col gap-tight">
  <span class="text-note font-label text-text-quiet">{say($lang, "decided_title")}</span>
  {#if newest === undefined}
    <p class="text-note text-text-disabled">…</p>
  {:else if newest.length === 0}
    <p class="text-note text-text-faint">{say($lang, "decided_none")}</p>
  {:else}
    <ul class="max-h-output overflow-auto text-note">
      {#each newest as decision (decision.item)}
        <li class="flex items-baseline gap-base border-b border-edge py-tight">
          <span class="shrink-0 text-text-faint">{clock($lang, decision.at)}</span>
          <span class="min-w-0 flex-1 truncate text-text-quiet">{decision.cluster.detail}</span>
          <span class={decision.verdict === "allow" ? "shrink-0 text-accent" : "shrink-0 text-alert"}>
            {say($lang, decision.verdict === "allow" ? "decided_allow" : "decided_deny")}
          </span>
        </li>
      {/each}
    </ul>
  {/if}
</div>
