<script lang="ts" module>
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The ledger, one page at a time and nothing folded away: each record
// is one readable line - what happened, where and by whom, when, and
// the few fields a person reads (`record/event.ts`) - and opens the
// record as it was written underneath, so the chain's own fields are
// one press away rather than the first thing a reader meets. Each line
// is a `parts/row` (client-SPEC 6 names this row as that component's
// seat).

import type { EventRecord } from "../../wire";
import { factsOf } from "./event";

// The facts as one line, the way a person skims a log.
function factLine(record: EventRecord): string {
  return factsOf(record.data)
    .map((fact) => `${fact.name}: ${fact.value}`)
    .join("   ·   ");
}
</script>

<script lang="ts">
  import { readAnswer } from "../../core/answered";
  import { fill, say } from "../../core/lang";
  import { clock, hhmmss } from "../../core/time";
  import { ui } from "../../ui";
  import type { Query, Seq } from "../../wire";
  import Button from "../parts/button.svelte";
  import Row, { RowList } from "../parts/row.svelte";
  import Tip from "../parts/tip.svelte";
  import Unanswered from "../parts/unanswered.svelte";
  import { whatHappened } from "./event";

  const u = ui();
  const lang = u.lang;

  let before = $state<Seq | null>(null);
  let open = $state<Seq | null>(null);

  const question = $derived<Query>({ history: { before, limit: 100 } });
  const history = $derived(u.conn.asking.ask(question));
  const read = $derived(readAnswer($history, (held) => ("history" in held ? held.history : undefined)));
  const answer = $derived(read.kind === "held" ? read.value : undefined);
</script>

{#if read.kind === "unavailable"}
  <Unanswered query={read.query} asked={question} />
{:else if answer === undefined}
  <p class="text-text-faint">…</p>
{:else}
  {#snippet ledgerRows()}
    {#each [...answer.records].reverse() as record (record.seq)}
      <Row
        primary={say($lang, whatHappened(record.kind))}
        secondary={factLine(record)}
        onOpen={() => {
          open = open === record.seq ? null : record.seq;
        }}
      >
        {#snippet status()}
          <span class="flex items-center gap-base whitespace-nowrap text-note">
            <span class="max-w-[32ch] truncate font-mono text-text-quiet">{record.addr ?? fill(say($lang, "rec_by"), { who: record.who })}</span>
            <span class="w-figure text-right font-mono text-text-faint">#{record.seq}</span>
            <Tip text={clock($lang, record.t)}>
              {#snippet children(hint)}
                <!-- svelte-ignore a11y_no_noninteractive_tabindex (the day rides one key away from the time, so the hint must be reachable by keyboard) -->
                <span tabindex="0" class="font-mono text-text-faint" aria-describedby={hint}>
                  {hhmmss(record.t)}
                </span>
              {/snippet}
            </Tip>
          </span>
        {/snippet}
      </Row>
      {#if open === record.seq}
        <li class="mb-snug flex flex-col gap-tight px-base">
          <span class="text-note text-text-faint">{say($lang, "rec_raw")}</span>
          <pre class="max-h-output overflow-auto rounded-card border border-edge bg-page p-base text-text-quiet">{JSON.stringify(record, null, 2)}</pre>
        </li>
      {/if}
    {/each}
  {/snippet}
  <div>
    {#if answer.earlier !== undefined && answer.earlier !== null}
      {@const earlier = answer.earlier}
      <div class="mb-base">
        <Button
          label={say($lang, "rec_earlier")}
          tone="secondary"
          onPress={() => {
            before = earlier;
          }}
        />
      </div>
    {/if}
    <!-- eslint-disable-next-line @typescript-eslint/no-unsafe-call (RowList is the keyboard walk `parts/row` exports as a snippet; svelte-check resolves its type where the lint's type graph does not) -->
    {@render RowList({ label: say($lang, "rec_ledger"), rows: ledgerRows })}
  </div>
{/if}
