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
  // Each row says the kind, the first line of the text and who sent it,
  // and leads to the sender's room (client D86).
  import { say } from "../../core/lang";
  import { toFragment } from "../../core/route";
  import { ago } from "../../core/time";
  import { kindSaid, waitingSaid } from "./inbox";
  import { called, residentAt } from "./naming";
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
    <h2 class="text-label font-label text-text-quiet">{waitingSaid($lang, waiting.length)}</h2>
    <ul>
      {#each waiting as line (line.id)}
        {@const sender = residentAt(line.from)}
        <li>
          <a
            href={sender === null ? undefined : toFragment({ kind: "talk", address: sender })}
            class="flex items-baseline gap-base rounded-control py-tight hover:bg-raised-hover"
          >
            <span class="shrink-0 text-text-quiet">{kindSaid($lang, line.kind)}</span>
            <span class="min-w-0 flex-1 truncate">{line.first_line ?? ""}</span>
            <span class="max-w-[30%] truncate text-text-faint">{sender === null ? line.from : called(sender, null, $lang)}</span>
            <span class="shrink-0 text-text-faint">{ago($lang, line.at, u.now())}</span>
          </a>
        </li>
      {/each}
    </ul>
  </section>
{/if}
