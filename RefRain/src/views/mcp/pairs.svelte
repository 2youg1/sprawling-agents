<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  import type { Pair } from "./draft";

  export interface PairTableProps {
    readonly caption: string;
    readonly rows: readonly Pair[];
    readonly onChange: (rows: readonly Pair[]) => void;
    readonly note?: string;
  }
</script>

<script lang="ts">
  import { say } from "../../core/lang";
  import { ui } from "../../ui";
  import Button from "../parts/button.svelte";
  import Field from "../parts/field.svelte";

  const { caption, rows, onChange, note }: PairTableProps = $props();

  const { lang } = ui();

  function written(at: number, pair: Pair): void {
    onChange(rows.map((row, index) => (index === at ? pair : row)));
  }

  function dropped(at: number): void {
    onChange(rows.filter((_, index) => index !== at));
  }
</script>

<div class="flex w-full min-w-0 flex-col gap-tight">
  <span class="text-note text-text-quiet">{caption}</span>
  {#each rows as row, at (at)}
    <div class="flex min-w-0 items-center gap-tight">
      <Field
        label="{caption} {say($lang, 'mcp_pair_name')}"
        labelling="hidden"
        mono
        placeholder={say($lang, "mcp_pair_name")}
        value={row.name}
        onInput={(value) => {
          written(at, { name: value, value: row.value });
        }}
      />
      <Field
        label="{caption} {say($lang, 'mcp_pair_value')}"
        labelling="hidden"
        mono
        placeholder={say($lang, "mcp_pair_value")}
        value={row.value}
        onInput={(value) => {
          written(at, { name: row.name, value });
        }}
      />
      <Button
        label={say($lang, "mcp_pair_remove")}
        tone="quiet"
        onPress={() => {
          dropped(at);
        }}
      />
    </div>
  {/each}
  <div class="flex items-center gap-base">
    <span class="shrink-0">
      <Button
        label={say($lang, "mcp_pair_add")}
        tone="quiet"
        onPress={() => {
          onChange([...rows, { name: "", value: "" }]);
        }}
      />
    </span>
    {#if note !== undefined}
      <span class="min-w-0 text-note text-text-faint">{note}</span>
    {/if}
  </div>
</div>
