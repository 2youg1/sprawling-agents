<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
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
  const { served, about, onChosen, state: lent }: ModelTableProps = $props();

  const owned = $state<TableState>(freshTable());
  const tableState = $derived(lent ?? owned);

  $effect(() => {
    onChosen(chosenRows(tableState, served));
  });

  const asked = $derived(
    about === null
      ? readable(undefined)
      : u.conn.asking.ask(figuresQuery(about, tableState, served)),
  );
  const figures = $derived(
    readAnswer($asked, (held) => ("model_figures" in held ? held.model_figures.models : undefined)),
  );

  const look = $derived(
    lookOf(tableState, served, figures.kind === "held" ? figures.value : [], $lang),
  );
</script>

<Look {...look} />
