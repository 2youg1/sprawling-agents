<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // How the attach form's fold is drawn, and nothing else: every word
  // arrives translated and every press arrives wired (`./advanced`,
  // `AdvancedLook`), so another look draws the same fold by taking the
  // same value.
  import Button from "../../parts/button.svelte";
  import Field from "../../parts/field.svelte";
  import Glyph from "../../parts/glyph.svelte";
  import Segmented from "../../parts/segmented.svelte";
  import type { AdvancedLook, BoxLook, PairTableLook } from "./advanced";

  const look: AdvancedLook = $props();
</script>

{#snippet box(field: BoxLook)}
  <Field
    label={field.label}
    kind={field.kind}
    mono={field.mono}
    value={field.value}
    placeholder={field.placeholder}
    onInput={field.input}
    {...(field.help === undefined ? {} : { help: field.help })}
    {...(field.step === undefined ? {} : { step: field.step })}
    {...(field.pattern === undefined ? {} : { pattern: field.pattern })}
  />
{/snippet}

{#snippet pairTable(table: PairTableLook)}
  <!-- A row keeps its boxes - and its `key`, which is what the list is
  keyed by - when another row is removed, so the box a person is typing
  in never becomes somebody else's row. -->
  <div class="flex flex-col gap-tight text-note text-text-quiet">
    {table.title}
    {#each table.rows as row (row.key)}
      <div class="flex items-center gap-tight">
        <Field label={row.name.label} labelling="hidden" mono value={row.name.value} onInput={row.name.input} />
        <Field label={row.value.label} labelling="hidden" mono value={row.value.value} onInput={row.value.input} />
        <button type="button" class="flex size-control shrink-0 items-center justify-center rounded-control text-text-quiet transition-[background-color] ease-leave hover:bg-raised hover:ease-arrive" onclick={row.remove.press}>
          <Glyph name="cross" size="sm" />
          <span class="sr-only">{row.remove.label}</span>
        </button>
      </div>
    {/each}
    <Button label={table.add.label} tone="quiet" onPress={table.add.press} />
  </div>
{/snippet}

<details class="flex flex-col gap-snug rounded-card bg-chrome px-base py-snug">
  <summary class="w-fit cursor-pointer rounded-control px-snug py-tight text-label text-text-quiet transition-[background-color] ease-leave hover:bg-raised hover:ease-arrive">
    {look.title}
  </summary>
  <div class="fields grid gap-snug">
    {#each look.naming as field (field.key)}
      <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression (a snippet call is the render itself; the typechecker types local snippet calls as returning void) -->
      {@render box(field)}
    {/each}
  </div>
  <div class="fields grid gap-snug">
    {#each look.tuning as field (field.key)}
      <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression (a snippet call is the render itself; the typechecker types local snippet calls as returning void) -->
      {@render box(field)}
    {/each}
  </div>
  <div class="flex flex-col gap-tight text-note text-text-quiet">
    {look.proxying.label}
    <Segmented label={look.proxying.label} options={look.proxying.options} held={look.proxying.held} onPick={look.proxying.pick} />
    <span class="text-text-faint">{look.proxying.help}</span>
    {#if look.proxying.note !== undefined}
      <span class="text-text-faint">{look.proxying.note}</span>
    {/if}
  </div>
  {#each look.tables as table (table.key)}
    <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression (a snippet call is the render itself; the typechecker types local snippet calls as returning void) -->
    {@render pairTable(table)}
  {/each}
</details>

<style>
  /* As many columns of boxes as the fold holds, each at least one
   * field track wide; a fold narrower than one track - the settings
   * sheet on a narrow window - gets one column as wide as the fold
   * rather than boxes drawn past its edge. */
  .fields {
    grid-template-columns: repeat(auto-fit, minmax(min(100%, var(--spacing-field-track)), 1fr));
  }
</style>
