<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->
<script lang="ts">
  // The signals queued for this room and not yet read by a run, asked
  // without taking them (`Query::InboxView`). The city bar counts them
  // across the city; this is where a person sees which they are.
  import { fill, say } from "../../core/lang";
  import { ago } from "../../core/time";
  import { ui } from "../../ui";
  import type { Address } from "../../wire";

  interface Props {
    readonly addr: Address;
  }

  const { addr }: Props = $props();
  const u = ui();
  const { lang } = u;

  const answer = $derived(u.conn.asking.ask({ inbox_view: { addr } }));
  const waiting = $derived.by(() => {
    const held = $answer;
    return held !== undefined && "inbox" in held ? held.inbox.waiting : [];
  });
</script>

{#if waiting.length > 0}
  <section class="my-base text-note" aria-label={say($lang, "inbox_waiting")}>
    <h2 class="text-label font-label text-text-quiet">
      {fill(say($lang, "inbox_count"), { n: String(waiting.length) })}
    </h2>
    <ul>
      {#each waiting as line (line.id)}
        <li class="flex items-baseline gap-base py-tight">
          <span class="min-w-0 flex-1 truncate">{line.kind}</span>
          <span class="truncate text-text-faint">{line.from}</span>
          <span class="shrink-0 text-text-faint">{ago($lang, line.at, u.now())}</span>
        </li>
      {/each}
    </ul>
  </section>
{/if}
