<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // The seat of the other half of choosing a model: what an endpoint
  // answered when it was asked for its model list, narrowed to the rows
  // a person ticked. An endpoint that serves two hundred rows, most of
  // them video and speech, is not a list anybody reads - it is a list
  // somebody filters.
  //
  // The seat holds what the person typed and ticked, hands the ticked
  // rows back, asks the city what an untouched box would register, and
  // draws whatever `./model_table.look.svelte` is. Which rows are
  // offered and what each figure means are decided in `./model_rows`,
  // what a press writes in `./model_table`, and nowhere here.
  import { readable } from "svelte/store";

  import { readAnswer } from "../../core/answered";
  import { ui } from "../../ui";
  import { chosenRows, figuresQuery, freshTable } from "./model_rows";
  import type { TableState } from "./model_rows";
  import { lookOf } from "./model_table";
  import Look from "./model_table.look.svelte";
  import type { ModelTableProps } from "./models";

  const u = ui();
  const lang = u.lang;
  const { served, about, onChosen }: ModelTableProps = $props();

  // Mutated through its properties only, so the binding itself is never
  // reassigned.
  const state = $state<TableState>(freshTable());

  $effect(() => {
    onChosen(chosenRows(state, served));
  });

  // The city's answer for every row (client D103). A table about no
  // endpoint asks nothing, and its boxes stay blank.
  const asked = $derived(about === null ? readable(undefined) : u.conn.asking.ask(figuresQuery(about, state, served)));
  const figures = $derived(
    readAnswer($asked, (held) => ("model_figures" in held ? held.model_figures.models : undefined)),
  );

  const look = $derived(lookOf(state, served, figures.kind === "held" ? figures.value : [], $lang));
</script>

<Look {...look} />
