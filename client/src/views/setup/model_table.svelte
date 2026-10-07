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
  // rows back, and draws whatever `./model_table.look.svelte` is. Which
  // rows are offered, what each figure means and what a press writes
  // are decided in `./model_table` and nowhere here.
  import { ui } from "../../ui";
  import { chosenRows, freshTable, lookOf } from "./model_table";
  import type { TableState } from "./model_table";
  import Look from "./model_table.look.svelte";
  import type { ModelTableProps } from "./models";

  const { lang } = ui();
  const { served, onChosen }: ModelTableProps = $props();

  // Mutated through its properties only, so the binding itself is never
  // reassigned.
  const state = $state<TableState>(freshTable());

  $effect(() => {
    onChosen(chosenRows(state, served));
  });

  const look = $derived(lookOf(state, served, $lang));
</script>

<Look {...look} />
