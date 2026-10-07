<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // How the web search card is drawn, and nothing else: every word
  // arrives translated, every press and every refusal arrives decided
  // (`./search`, `SearchLook`), so another look draws the same card by
  // taking the same value (client D94). Each service folds its account
  // editor, seated with the roster the wiring built for it.
  import Accounts from "../providers/accounts.svelte";
  import Button from "../../parts/button.svelte";
  import Field from "../../parts/field.svelte";
  import Notice from "../../parts/notice.svelte";
  import Segmented from "../../parts/segmented.svelte";
  import type { SearchLook } from "./search";

  const look: SearchLook = $props();
</script>

<section class="flex min-w-0 flex-col gap-snug rounded-card bg-raised px-base py-snug" aria-label={look.heading}>
  <div class="flex flex-col gap-hair">
    <h3 class="text-label font-label text-text">{look.heading}</h3>
    <p class="text-note text-text-faint">{look.intro}</p>
  </div>
  <Segmented label={look.choice.label} tone="accent" options={look.choice.options} held={look.choice.held} onPick={look.choice.pick} />
  <p class="text-note text-text-faint">{look.defaultAt}</p>
  {#if look.override !== undefined}
    <p class="text-note text-text-quiet">{look.override}</p>
  {/if}
  {#if look.noFallback !== undefined}
    <p class="text-note text-text-quiet">{look.noFallback}</p>
  {/if}
  <p class="text-note text-text-faint">{look.confidential}</p>
  {#if look.suppliers.length > 0}
    <h4 class="font-label text-text-quiet">{look.suppliersHeading}</h4>
    <ul class="flex flex-col gap-snug">
      {#each look.suppliers as row (row.id)}
        <li class="flex min-w-0 flex-col gap-tight rounded-card bg-chrome px-base py-snug text-note">
          <div class="flex min-w-0 flex-wrap items-center gap-snug">
            <span class={["font-mono", row.selected ? "text-accent" : "text-text"]}>{row.id}</span>
            <span class="min-w-0 flex-1 truncate font-mono text-text-faint">{row.url}</span>
            <span class="font-mono text-text-quiet">{row.remote}</span>
            <span class="flex flex-wrap justify-end gap-tight">
              {#each row.controls as control (control.key)}
                <Button
                  label={control.label}
                  tone="quiet"
                  onPress={control.press}
                  {...(control.why === undefined ? {} : { why: control.why })}
                />
              {/each}
            </span>
          </div>
          <details>
            <summary class="text-text-quiet">{row.accounts}</summary>
            <div class="pt-snug">
              <Accounts roster={row.roster} />
            </div>
          </details>
        </li>
      {/each}
    </ul>
  {/if}
  {#if look.form !== undefined}
    {@const form = look.form}
    <details open={form.open}>
      <summary class="font-label text-text-quiet">{form.title}</summary>
      <form
        class="flex flex-col gap-snug pt-snug"
        onsubmit={(event) => {
          event.preventDefault();
          form.save.press();
        }}
      >
        {#each form.fields as field (field.name)}
          <Field label={field.label} value={field.value} onInput={field.input} mono disabled={field.disabled} />
        {/each}
        <span class="flex flex-wrap gap-snug">
          <Button type="submit" tone="secondary" label={form.save.label} {...(form.save.why === undefined ? {} : { why: form.save.why })} />
          {#if form.cancel !== undefined}
            <Button tone="quiet" label={form.cancel.label} onPress={form.cancel.press} />
          {/if}
        </span>
      </form>
    </details>
  {/if}
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
</section>
