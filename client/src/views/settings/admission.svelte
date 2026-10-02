<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  // What bounds a run's work, explained once for the developer (refrain
  // roadmap Q6): three values that are chosen apart and combine - what a
  // run may write, what evidence its work must carry before a merge lets
  // it in, and whether that work lands at all. An explanation rather
  // than a control: each is chosen per dispatch where the work is
  // handed out, and the city records the choice on the run it starts,
  // so a standing control here would be a second place that decides it.
  // The spellings are the wire's (`WriteLimit`, `AdmissionRequirement`,
  // `LandingPolicy`), drawn as code because a person types them.

  import { say } from "../../core/lang";
  import type { Key } from "../../core/lang";
  import { ui } from "../../ui";
  import type { AdmissionRequirement, LandingPolicy, WriteLimit } from "../../wire";

  const { lang } = ui();

  interface Row {
    readonly term: Key;
    readonly says: Key;
    readonly values: readonly string[];
  }

  const WRITES: readonly WriteLimit[] = ["full", "create"];
  const REQUIRES: readonly AdmissionRequirement[] = ["standing", "tested", "contract_kept", "double_validated"];
  const LANDINGS: readonly LandingPolicy[] = ["ordinary", "experiment"];

  const ROWS: readonly Row[] = [
    { term: "admission_write", says: "admission_write_says", values: WRITES },
    { term: "admission_require", says: "admission_require_says", values: REQUIRES },
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
