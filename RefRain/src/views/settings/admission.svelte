<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  import { ADMISSIONS, LANDINGS, WRITE_LIMITS } from "../../core/commands";
  import { say } from "../../core/lang";
  import type { Key } from "../../core/lang";
  import { ui } from "../../ui";

  const { lang } = ui();

  interface Row {
    readonly term: Key;
    readonly says: Key;
    readonly values: readonly string[];
  }

  const ROWS: readonly Row[] = [
    { term: "admission_write", says: "admission_write_says", values: WRITE_LIMITS },
    { term: "admission_require", says: "admission_require_says", values: ADMISSIONS },
    { term: "admission_landing", says: "admission_landing_says", values: LANDINGS },
  ];
</script>

<div class="flex min-w-0 flex-col gap-snug rounded-card bg-raised px-base py-snug">
  <div class="flex flex-col gap-hair">
    <span class="text-label font-label text-text">{say($lang, "admission_title")}</span>
    <p class="text-note text-text-faint">{say($lang, "admission_note")}</p>
  </div>
  <dl class="flex flex-col gap-snug">
    {#each ROWS as row (row.term)}
      <div class="flex flex-col gap-hair border-t border-edge pt-snug">
        <dt class="text-note font-label text-text">{say($lang, row.term)}</dt>
        <dd class="text-note text-text-quiet">{say($lang, row.says)}</dd>
        <dd class="flex flex-wrap gap-snug font-mono text-note text-text-faint">
          {#each row.values as value (value)}
            <code>{value}</code>
          {/each}
        </dd>
      </div>
    {/each}
  </dl>
  <p class="text-note text-text-faint">{say($lang, "admission_apart")}</p>
</div>
