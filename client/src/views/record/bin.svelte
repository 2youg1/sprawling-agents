<script lang="ts" module>
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The recycle bin, where every row states its own way back. A row
// without a way back cannot be constructed upstream, so every row here
// carries one and the page draws it rather than a guess.

import { say } from "../../core/lang";
import type { Lang } from "../../core/lang";
import type { DiscardLine } from "../../wire";

// How this row gets back, in one line. The three arms are the three
// storage authorities the wire names and no fourth exists.
function way(lang: Lang, row: DiscardLine): string {
  const restoration = row.restoration;
  if (restoration === null || restoration === undefined) return "";
  if ("tracked" in restoration) return `${say(lang, "bin_tracked")} ${restoration.tracked}`;
  if ("interred" in restoration) return `${say(lang, "bin_interred")} ${restoration.interred}`;
  return `${say(lang, "bin_rebuildable")} ${restoration.rebuildable.reason}`;
}

// The identity of one row: what was discarded when. A path can be
// discarded twice, and the moment is what tells the two apart.
function keyOf(row: DiscardLine): string {
  return `${String(row.at)}\u0000${row.path}`;
}
</script>

<script lang="ts">
  import { QUERIES } from "../../core/asking";
  import { restoreDiscard } from "../../core/commands";
  import { clock } from "../../core/time";
  import { ui } from "../../ui";
  import Path from "../parts/path.svelte";

  const u = ui();
  const lang = u.lang;
  const send = u.send;
  const held = u.conn.asking.ask(QUERIES.discards);

  const rows = $derived.by(() => {
    const answer = $held;
    return answer !== undefined && "discards" in answer ? answer.discards.rows : undefined;
  });
</script>

{#if rows === undefined}
  <p class="text-text-disabled">…</p>
{:else if rows.length === 0}
  <p class="text-text-faint">{say($lang, "bin_empty")}</p>
{:else}
  <ul class="text-note">
    {#each rows as row (keyOf(row))}
      <li class="settled-row border-b border-edge py-snug">
        <div class="flex items-center gap-base">
          <span class="min-w-0 flex-1">
            <Path path={row.path} />
          </span>
          <span class="text-text-faint">{clock($lang, row.at)}</span>
          <span class={row.restored ? "text-text-disabled" : "text-alert"}>
            {row.restored ? say($lang, "bin_restored") : say($lang, "bin_gone")}
          </span>
          {#if !row.restored && row.restoration !== null && row.restoration !== undefined && "tracked" in row.restoration}
            {@const restoration = row.restoration}
            <button
              type="button"
              class="rounded-control px-snug text-text-quiet hover:text-text"
              onclick={() => {
                send(restoreDiscard(restoration));
              }}
            >
              {say($lang, "bin_restore")}
            </button>
          {/if}
        </div>
        <div class="summary mt-tight truncate font-mono text-text-faint">{way($lang, row)}</div>
      </li>
    {/each}
  </ul>
{/if}
