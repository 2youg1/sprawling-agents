<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
</script>

<script lang="ts">
  import { configureContext } from "../../core/commands";
  import { fill, say } from "../../core/lang";
  import { ui } from "../../ui";
  import { readAnswer } from "../../core/answered";
  import type { Address, Query, SettledSecond } from "../../wire";
  import Button from "../parts/button.svelte";
  import Field from "../parts/field.svelte";
  import Unanswered from "../parts/unanswered.svelte";
  import { HALL, useBuildings } from "../shared/buildings";

  const u = ui();
  const lang = u.lang;
  const buildings = useBuildings();

  let addr = $state<Address>(HALL);

  let box = $state("");
  let edited = $state(false);

  const question = $derived<Query>({ config: { addr } });
  const config = $derived(u.conn.asking.ask(question));
  const read = $derived(
    readAnswer($config, (answer) => ("config" in answer ? answer.config.second : undefined)),
  );
  const settled = $derived(read.kind === "held" ? read.value : undefined);
  const onDisk = $derived(
    settled === undefined || settled.from === "default" ? "" : String(settled.percent),
  );

  let was: Address | undefined = undefined;
  $effect(() => {
    const at = addr;
    const text = onDisk;
    if (!edited || was !== at) {
      edited = false;
      box = text;
    }
    was = at;
  });

  const span = (domain: SettledSecond["domain"]): string =>
    fill(say($lang, "context_second_range"), {
      min: String(domain.min),
      max: String(domain.max),
    });

  function save(): void {
    const percent = Number(box);
    if (!Number.isInteger(percent)) return;
    if (u.send(configureContext(addr, percent))) {
      edited = false;
    }
  }
</script>

<div class="flex flex-col gap-tight rounded-card bg-raised px-base py-snug">
  <span class="text-label font-label text-text">{say($lang, "context_second")}</span>
  <p class="text-note text-text-faint">{say($lang, "context_second_note")}</p>
  <select
    class="h-control w-tree max-w-full rounded-control border border-edge-input bg-raised px-base text-body text-text"
    aria-label={say($lang, "context_second_building")}
    value={addr}
    onchange={(event) => {
      const picked = $buildings.find((each) => each === event.currentTarget.value);
      if (picked !== undefined) addr = picked;
    }}
  >
    {#each $buildings as each (each)}
      <option value={each}>{each === HALL ? say($lang, "city_hall") : each}</option>
    {/each}
  </select>
  <Field
    label={say($lang, "context_second")}
    labelling="hidden"
    kind="number"
    step={1}
    suffix={say($lang, "context_second_unit")}
    mono
    {...settled?.from === "default" ? { placeholder: String(settled.percent) } : {}}
    value={box}
    onInput={(next: string) => {
      box = next;
      edited = true;
    }}
  />
  {#if read.kind === "unavailable"}
    <Unanswered query={read.query} asked={question} />
  {/if}
  <div class="flex items-center gap-base">
    <div class="shrink-0">
      <Button
        label={say($lang, "context_second_save")}
        tone="primary"
        {...edited ? {} : { why: say($lang, "context_second_unchanged") }}
        onPress={save}
      />
    </div>
    {#if settled !== undefined}
      <span class="min-w-0 text-note text-text-faint">{span(settled.domain)}</span>
    {/if}
  </div>
</div>
