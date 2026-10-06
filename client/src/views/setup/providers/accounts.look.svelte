<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // How one endpoint's account editor is drawn, and nothing else: every
  // word arrives translated, every press and every refusal arrives
  // decided (`./accounts`, `AccountsLook`), so another look - one built
  // from a component library - draws the same editor by taking the same
  // value (client D92). The two holds are put on the elements focus
  // returns to: the heading after a removal, the box around a move
  // control after its row moved.
  import Button from "../../parts/button.svelte";
  import Field from "../../parts/field.svelte";
  import Notice from "../../parts/notice.svelte";
  import Segmented from "../../parts/segmented.svelte";
  import type { AccountsLook, FieldLook } from "./accounts";

  const look: AccountsLook = $props();
  const standing = $derived(look.fields.filter((field) => !field.folded));
  const folded = $derived(look.fields.filter((field) => field.folded));
</script>

{#snippet drawField(field: FieldLook)}
  <Field
    label={field.label}
    value={field.value}
    onInput={field.input}
    kind={field.secret ? "password" : "text"}
    mono={field.mono}
    disabled={field.disabled}
    {...(field.help === undefined ? {} : { help: field.help })}
  />
{/snippet}

<div class="flex min-w-0 flex-col gap-snug">
  <h4 tabindex="-1" class="font-label text-text-quiet" {@attach look.holdHeading}>{look.heading}</h4>
  <p class="text-note text-text-faint">{look.order}</p>
  {#if look.legacy !== undefined}
    <p class="text-note text-text-quiet">{look.legacy}</p>
  {/if}
  {#if look.rows.length > 0}
    <ol class="flex flex-col gap-tight">
      {#each look.rows as row (row.id)}
        <li class="flex min-w-0 flex-wrap items-center gap-snug text-note">
          <span class="w-figure text-right font-mono text-text-faint">{row.place}</span>
          <span class="font-mono text-text">{row.id}</span>
          <span class={row.wanting ? "text-alert" : "text-text-faint"}>{row.key}</span>
          <span class="flex flex-1 flex-wrap justify-end gap-tight">
            {#each row.controls as control (control.key)}
              <span {@attach control.hold}>
                <Button
                  label={control.label}
                  tone="quiet"
                  onPress={control.press}
                  {...(control.why === undefined ? {} : { why: control.why })}
                />
              </span>
            {/each}
          </span>
        </li>
      {/each}
    </ol>
  {/if}
  {#if look.retries !== undefined}
    <div class="flex flex-col gap-tight">
      <span class="text-note text-text-quiet">{look.retries.label}</span>
      <Segmented label={look.retries.label} options={look.retries.options} held={look.retries.held} onPick={look.retries.pick} />
      {#if look.retries.fallback !== undefined}
        <p class="text-note text-text-faint">{look.retries.fallback}</p>
      {/if}
    </div>
  {/if}
  <form
    class="flex flex-col gap-snug"
    onsubmit={(event) => {
      event.preventDefault();
      look.save.press();
    }}
  >
    <p class="font-label text-text-quiet">{look.title}</p>
    {#each standing as field (field.name)}
      <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression (a snippet call is the render itself; the typechecker types local snippet calls as returning void) -->
      {@render drawField(field)}
    {/each}
    <details>
      <summary class="text-note text-text-quiet">{look.more}</summary>
      <div class="flex flex-col gap-snug pt-snug">
        {#each folded as field (field.name)}
          <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression (a snippet call is the render itself; the typechecker types local snippet calls as returning void) -->
          {@render drawField(field)}
        {/each}
      </div>
    </details>
    <p class="text-note text-text-faint">{look.route}</p>
    {#if look.note !== undefined}
      <p role="alert" class="text-note text-alert">{look.note}</p>
    {/if}
    {#if look.refused !== undefined}
      <Notice
        seat="inline"
        weight="alert"
        action={look.refused.action}
        code={look.refused.code}
        subject={look.refused.subject}
        recovery={look.refused.recovery}
      />
    {/if}
    <span class="flex flex-wrap gap-snug">
      <Button
        type="submit"
        tone="secondary"
        label={look.save.label}
        loading={look.save.loading}
        {...(look.save.why === undefined ? {} : { why: look.save.why })}
      />
      {#if look.cancel !== undefined}
        <Button tone="quiet" label={look.cancel.label} onPress={look.cancel.press} />
      {/if}
    </span>
  </form>
</div>
