<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  // Where the context reminder's second rung sits: a whole percent of
  // the window. The span and its reason are
  // `kernel::config::SecondThreshold`'s one construction point - thirty-one
  // through ninety, so the line about a handoff can still be said
  // before the window closes - and this page states the span the city
  // answers with rather than judging it: the city's refusal comes back
  // carrying the span, and a page that enforced it here, or spelled it
  // itself, would be the second place the rule lives.
  //
  // A box left empty is the city's own default, written as nothing at
  // all. What is in force is what `Query::Config` answers - the file's
  // word, read fresh, and the layer it came from - not this page's
  // memory of a save. The default is one of those layers, so the page
  // draws the city's figure as the empty box's hint rather than keeping
  // a copy of it.
  //
  // The card stands in the settings tree's run group and picks its
  // building itself, the way the rules group does (client 4-66, D78).
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
  // What is in force here and which layer said it, once the city has
  // answered; a city that could not answer is said so, not drawn as a
  // rung nobody set.
  const read = $derived(readAnswer($config, (answer) => ("config" in answer ? answer.config.second : undefined)));
  const settled = $derived(read.kind === "held" ? read.value : undefined);
  // What a file says, or an empty box where no file states the rung -
  // which is the default in force rather than a gap.
  const onDisk = $derived(settled === undefined || settled.from === "default" ? "" : String(settled.percent));

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
    class="h-control w-tree max-w-full rounded-control border border-edge-input bg-page px-base text-body text-text"
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
    {...(settled?.from === "default" ? { placeholder: String(settled.percent) } : {})}
    value={box}
    onInput={(next: string) => {
      box = next;
      edited = true;
    }}
  />
  {#if read.kind === "unavailable"}
    <Unanswered query={read.query} asked={question} />
  {/if}
  <!-- The key keeps its one-line name: the domain sentence beside it
    is what wraps when the column is narrow. -->
  <div class="flex items-center gap-base">
    <div class="shrink-0">
      <Button
        label={say($lang, "context_second_save")}
        tone="primary"
        {...(edited ? {} : { why: say($lang, "context_second_unchanged") })}
        onPress={save}
      />
    </div>
    {#if settled !== undefined}
      <span class="min-w-0 text-note text-text-faint">{span(settled.domain)}</span>
    {/if}
  </div>
</div>
