<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  // The uses of one skill or one server's tool, as the usage panels on
  // the skill page and the MCP page both draw them: the count and the
  // last use on one line, the counts per UTC day, and every use - run,
  // room, moment, the part read, how the call ended - folded under a
  // disclosure, because a skill read a thousand times would otherwise
  // push every other row off the page. The city folds the table
  // (`Query::SkillUsage`, `Query::McpUsage`); this part only draws it.
  import type { Key } from "../../core/lang";
  import type { DayCount, UseOutcome } from "../../wire";

  export interface UseRow {
    readonly run: string;
    readonly resident: string | null;
    readonly at: number;
    readonly part: string | null;
    readonly outcome: UseOutcome;
  }

  export function outcomeWord(outcome: UseOutcome): Key {
    switch (outcome) {
      case "ok":
        return "usage_outcome_ok";
      case "failed":
        return "usage_outcome_failed";
      case "unknown":
        return "usage_outcome_unknown";
    }
  }

  export function dayLine(days: readonly DayCount[]): string {
    // wording-ok: a calendar day and a count, spelled the same in every language.
    return days.map((day) => `${day.day} ${String(day.count)}`).join(" · ");
  }
</script>

<script lang="ts">
  import { fill, say } from "../../core/lang";
  import { clock } from "../../core/time";
  import { ui } from "../../ui";

  interface Props {
    readonly uses: readonly UseRow[];
    readonly days: readonly DayCount[];
  }

  const { uses, days }: Props = $props();
  const { lang } = ui();
  const last = $derived(uses.at(-1));
</script>

{#if last === undefined}
  <p class="text-note text-text-faint">{say($lang, "usage_never")}</p>
{:else}
  <p class="text-note text-text-quiet">
    {fill(say($lang, "usage_count"), { n: String(uses.length), when: clock($lang, last.at) })}
  </p>
  <p class="font-mono text-note text-text-faint">{dayLine(days)}</p>
  <details class="text-note">
    <summary class="cursor-pointer text-text-faint hover:text-text">{say($lang, "usage_every_use")}</summary>
    <ul class="mt-tight flex flex-col gap-tight">
      {#each uses as used, index (index)}
        <li class="flex min-w-0 items-baseline gap-base text-text-faint">
          <span class="shrink-0">{clock($lang, used.at)}</span>
          <span class="min-w-0 truncate font-mono">{used.resident ?? used.run}</span>
          {#if used.part !== null}
            <span class="min-w-0 truncate font-mono">{used.part}</span>
          {/if}
          <span class="shrink-0">{say($lang, outcomeWord(used.outcome))}</span>
        </li>
      {/each}
    </ul>
  </details>
{/if}
