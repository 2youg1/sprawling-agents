<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  // The tabs in the states the pages draw them in: a lens whose panel
  // holds a control and a count beside another lens's name, as the run
  // page and the record draw them; a panel of plain text, which is a
  // Tab stop of its own; and the run page's seven lenses in a phone's
  // column, where the strip wraps instead of running past its frame.
  // Each case holds the lens a person picked in it.

  import type { RunLens } from "../../../core/route";
  import type { Key } from "../../../core/lang";

  const WORDS: readonly (readonly [RunLens, Key])[] = [
    ["time", "run_time"],
    ["turns", "run_turns"],
    ["monitor", "run_monitor"],
    ["prompt", "run_prompt"],
    ["context", "run_context"],
    ["changes", "run_changes"],
    ["evidence", "run_evidence"],
  ];
</script>

<script lang="ts">
  import { say } from "../../../core/lang";
  import { ui } from "../../../ui";
  import Badge from "../../parts/badge.svelte";
  import Button from "../../parts/button.svelte";
  import Tabs from "../../parts/tabs.svelte";
  import type { Lens } from "../../parts/tabs";
  import Case from "../case.svelte";

  const { lang } = ui();

  const all = $derived(WORDS.map(([id, key]): Lens => ({ id, label: say($lang, key) })));
  const three = $derived(all.filter((lens) => lens.id === "time" || lens.id === "turns" || lens.id === "changes"));

  let held = $state("turns");
  let plain = $state("time");
  let narrow = $state("evidence");
</script>

{#snippet panel(lens: Lens)}
  {#if lens.id === "turns"}
    <div class="flex items-center gap-snug pt-snug">
      <Button label={say($lang, "run_evidence")} tone="secondary" onPress={() => undefined} />
    </div>
  {:else if lens.id === "time"}
    <p class="pt-snug text-note text-text-quiet">{say($lang, "run_no_turns")}</p>
  {:else}
    <p class="pt-snug text-note text-text-quiet">{say($lang, "commits_empty")}</p>
  {/if}
{/snippet}

{#snippet mark(lens: Lens)}
  {#if lens.id === "turns"}
    <Badge text="12" />
  {/if}
{/snippet}

<Case label="tabs · three lenses, a control in the panel and a count beside a name">
  <Tabs
    label={say($lang, "run_lenses")}
    lenses={three}
    current={held}
    onPick={(id) => {
      held = id;
    }}
    {panel}
    {mark}
  />
</Case>

<Case label="tabs · a panel of plain text is a stop of its own">
  <Tabs
    label={say($lang, "run_lenses")}
    lenses={three}
    current={plain}
    onPick={(id) => {
      plain = id;
    }}
    {panel}
  />
</Case>

<Case label="tabs · seven lenses at a phone's width" width={390}>
  <Tabs
    label={say($lang, "run_lenses")}
    lenses={all}
    current={narrow}
    onPick={(id) => {
      narrow = id;
    }}
    {panel}
  />
</Case>
