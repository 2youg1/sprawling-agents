<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // A second look for the account editor, written the way a component
  // library writes one: native elements, no utility classes, every
  // style in its own block and read from the theme's tokens. It takes
  // the same `AccountsLook` as the shipped look and nothing else, so
  // drawing the editor with it proves the wiring and the focus returns
  // live in `accounts.ts` and the seat, not in the shipped markup
  // (client D92).
  import type { AccountsLook, ControlLook, FieldLook } from "../../../../src/views/setup/providers/accounts";

  const look: AccountsLook = $props();
  const uid = $props.id();
  const standing = $derived(look.fields.filter((field) => !field.folded));
  const folded = $derived(look.fields.filter((field) => field.folded));

  function pressed(control: { readonly why: string | undefined; readonly press: () => void }): void {
    if (control.why === undefined) control.press();
  }
</script>

{#snippet drawControl(control: ControlLook, row: string)}
  <button
    type="button"
    class="control"
    aria-disabled={control.why === undefined ? undefined : "true"}
    aria-describedby={control.why === undefined ? undefined : `${uid}-${row}-${control.key}`}
    onclick={() => {
      pressed(control);
    }}>{control.label}</button
  >
  {#if control.why !== undefined}
    <span id={`${uid}-${row}-${control.key}`} class="why">{control.why}</span>
  {/if}
{/snippet}

{#snippet drawField(field: FieldLook)}
  <label class="field">
    <span>{field.label}</span>
    <input
      type={field.secret ? "password" : "text"}
      class:mono={field.mono}
      value={field.value}
      disabled={field.disabled}
      aria-describedby={field.help === undefined ? undefined : `${uid}-${field.name}-help`}
      oninput={(event) => {
        field.input(event.currentTarget.value);
      }}
    />
    {#if field.help !== undefined}
      <small id={`${uid}-${field.name}-help`}>{field.help}</small>
    {/if}
  </label>
{/snippet}

<section class="editor">
  <h4 tabindex="-1" {@attach look.holdHeading}>{look.heading}</h4>
  <p class="quiet">{look.order}</p>
  {#if look.legacy !== undefined}
    <p>{look.legacy}</p>
  {/if}
  {#if look.rows.length > 0}
    <ol class="rows">
      {#each look.rows as row (row.id)}
        <li class="row">
          <span class="place">{row.place}</span>
          <span class="mono">{row.id}</span>
          <span class:wanting={row.wanting}>{row.key}</span>
          <span class="controls">
            {#each row.controls as control (control.key)}
              <span {@attach control.hold}>
                <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression (a snippet call is the render itself; the typechecker types local snippet calls as returning void) -->
                {@render drawControl(control, row.id)}
              </span>
            {/each}
          </span>
        </li>
      {/each}
    </ol>
  {/if}
  {#if look.retries !== undefined}
    <fieldset class="retries">
      <legend>{look.retries.label}</legend>
      {#each look.retries.options as option (option.value)}
        <label>
          <input
            type="radio"
            name={`${uid}-retries`}
            checked={look.retries.held === option.value}
            disabled={option.why !== undefined}
            onchange={() => {
              look.retries?.pick(option.value);
            }}
          />
          {option.label}
        </label>
      {/each}
      {#if look.retries.fallback !== undefined}
        <small>{look.retries.fallback}</small>
      {/if}
    </fieldset>
  {/if}
  <form
    onsubmit={(event) => {
      event.preventDefault();
      pressed(look.save);
    }}
  >
    <h5>{look.title}</h5>
    {#each standing as field (field.name)}
      <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression (a snippet call is the render itself; the typechecker types local snippet calls as returning void) -->
      {@render drawField(field)}
    {/each}
    <details>
      <summary>{look.more}</summary>
      {#each folded as field (field.name)}
        <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression (a snippet call is the render itself; the typechecker types local snippet calls as returning void) -->
        {@render drawField(field)}
      {/each}
    </details>
    <p class="quiet">{look.route}</p>
    {#if look.note !== undefined}
      <p role="alert" class="wanting">{look.note}</p>
    {/if}
    {#if look.refused !== undefined}
      <p role="alert" class="wanting">{look.refused.code} · {look.refused.recovery}</p>
    {/if}
    <span class="controls">
      <button
        type="submit"
        class="control"
        aria-busy={look.save.loading}
        aria-disabled={look.save.why === undefined && !look.save.loading ? undefined : "true"}>{look.save.label}</button
      >
      {#if look.save.why !== undefined}
        <span class="why">{look.save.why}</span>
      {/if}
      {#if look.cancel !== undefined}
        <button type="button" class="control" onclick={look.cancel.press}>{look.cancel.label}</button>
      {/if}
    </span>
  </form>
</section>

<style>
  .editor,
  form,
  .field {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-tight);
    min-width: 0;
  }
  .editor {
    color: var(--color-text);
    font-size: var(--text-note);
  }
  .retries {
    display: flex;
    flex-wrap: wrap;
    gap: var(--spacing-snug);
    border: 0;
    padding: 0;
  }
  .rows {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-tight);
  }
  .row {
    display: grid;
    grid-template-columns: var(--spacing-figure) minmax(0, 1fr) minmax(0, 1fr);
    gap: var(--spacing-snug);
    align-items: center;
    border-left: var(--spacing-hair) solid var(--color-edge);
    padding-left: var(--spacing-snug);
  }
  .controls {
    display: flex;
    flex-wrap: wrap;
    gap: var(--spacing-tight);
    grid-column: 1 / -1;
  }
  .control {
    border: var(--spacing-hair) solid var(--color-accent);
    border-radius: var(--radius-control);
    padding: 0 var(--spacing-snug);
    color: var(--color-text);
    background: var(--color-raised);
    transition: background-color var(--transition-duration-short) var(--ease-arrive);
  }
  .control:hover {
    background: var(--color-raised-hover);
  }
  .control[aria-disabled="true"] {
    color: var(--color-text-disabled);
    border-color: var(--color-disabled);
  }
  .quiet,
  .why,
  small {
    color: var(--color-text-quiet);
  }
  .wanting {
    color: var(--color-alert);
  }
  input {
    border: var(--spacing-hair) solid var(--color-edge-input);
    border-radius: var(--radius-control);
    padding: 0 var(--spacing-snug);
    background: var(--color-g1);
    color: var(--color-text);
  }
  .mono,
  input {
    font-family: var(--font-mono);
  }
  .place {
    text-align: right;
  }
</style>
