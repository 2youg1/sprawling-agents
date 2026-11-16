<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  import { fill, say } from "../../core/lang";
  import { FIRST, bandsOf, resultsOf } from "../../core/results";
  import type { Outcome } from "../../core/results";
  import { ui } from "../../ui";
  import Segmented from "../parts/segmented.svelte";
  import type { Choice } from "../parts/segmented";
  import { resultsLookOf } from "./results";
  import Look from "./results.look.svelte";

  type Tab = "all" | Outcome;

  const u = ui();
  const { lang } = u;
  const belief = u.conn.belief;

  let tab = $state<Tab>("all");

  const runs = $derived(Object.values($belief.runs));
  const groups = $derived(resultsOf(runs, FIRST));
  const listed = $derived(groups.reduce((sum, group) => sum + group.total, 0));
  const tabs = $derived<readonly Choice<Tab>[]>([
    {
      value: "all",
      label: fill(say($lang, "results_tab"), {
        name: say($lang, "results_all"),
        n: String(listed),
      }),
    },
    ...groups
      .filter((group) => group.outcome !== "ended" || group.total > 0)
      .map((group) => ({
        value: group.outcome,
        label: fill(say($lang, "results_tab"), {
          name: say($lang, `results_${group.outcome}`),
          n: String(group.total),
        }),
      })),
  ]);
  const bands = $derived(
    bandsOf(
      groups
        .filter((group) => tab === "all" || group.outcome === tab)
        .flatMap((group) => group.first),
      u.now(),
    ),
  );
</script>

<div class="mx-auto w-full max-w-page">
  <p class="text-note text-text-quiet">
    {fill(say($lang, "results_city"), {
      runs: String(runs.length),
      rest: String(runs.length - listed),
    })}
  </p>
  <div class="mt-snug">
    <Segmented
      label={say($lang, "results_which")}
      options={tabs}
      held={tab}
      onPick={(value) => {
        tab = value;
      }}
    />
  </div>
  <div class="mt-wide">
    <Look {...resultsLookOf(bands, $lang)} />
  </div>
</div>
