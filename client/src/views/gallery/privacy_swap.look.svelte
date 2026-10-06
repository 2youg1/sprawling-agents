<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  // A second look for a privacy entry, written the way a UI library's
  // card is: its own markup, its own `<style>`, no utility classes, and
  // nothing from `core/` or the wire. It takes exactly `EntryLook`, so
  // the gallery can draw a real entry through it beside the shipped look
  // (`setup/privacy/entry.look.svelte`): the typecheck holds the props,
  // the wiring tests never import a look, and `cargo xtask render`
  // measures this one as it measures the shipped one. If a swap lost the
  // entry's names, roles or alignment, the gate would say so here.

  import Button from "../parts/button.svelte";
  import type { EntryLook } from "../setup/privacy/entry";

  const look: EntryLook = $props();
  const titleId = $derived(`${look.id}-swap-title`);
</script>

<article id="{look.id}-swap" tabindex="-1" aria-labelledby={titleId} class="swap">
  <header class="swap-head">
    <h4 id={titleId} class="swap-title">{look.title}</h4>
    <div class="swap-presses">
      {#each look.presses as press (press.label)}
        <Button label={press.label} tone="primary" onPress={press.onPress} />
      {/each}
    </div>
  </header>
  <p class="swap-quiet swap-break">{look.lines.label}: {look.lines.text}</p>
  <p class="swap-strong">{look.overlooked.label}: {look.overlooked.text}</p>
  <ul class="swap-facts">
    {#each look.facts as fact (fact.label)}
      <li><span class="swap-quiet">{fact.label}</span> <span class="swap-break">{fact.value}</span></li>
    {/each}
  </ul>
  <p class={look.fitWeight === "alert" ? "swap-alert" : "swap-quiet"}>{look.fit}</p>
  {#if look.effect !== null}
    <p class="swap-alert">{look.effect}</p>
  {/if}
  <details class="swap-more">
    <summary>{look.editions.label}</summary>
    {#each look.notes as note (note.label)}
      <p><span class="swap-quiet">{note.label}</span> {note.text}</p>
    {/each}
    <p>{look.editions.text}</p>
    <p class="swap-quiet">{look.editionsNote}</p>
    <p class="swap-break">{look.target.label}: {look.target.text}</p>
  </details>
  <p role="status" class={look.status?.weight === "alert" ? "swap-alert" : "swap-quiet"}>{look.status?.text ?? ""}</p>
</article>

<style>
  .swap {
    display: flex;
    min-width: 0;
    flex-direction: column;
    gap: var(--spacing-tight);
    border: 1px solid var(--color-edge-input);
    border-radius: var(--radius-card);
    padding: var(--spacing-snug) var(--spacing-base);
    font-size: var(--text-note);
    color: var(--color-text);
    transition: border-color var(--transition-duration-panel) var(--ease-arrive);
  }
  .swap:hover {
    border-color: var(--color-accent);
  }
  .swap-head {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
    gap: var(--spacing-snug);
  }
  .swap-title {
    min-width: 0;
    font-size: var(--text-label);
  }
  .swap-presses {
    display: flex;
    gap: var(--spacing-tight);
  }
  .swap-facts {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-tight);
  }
  .swap-quiet {
    color: var(--color-text-quiet);
  }
  .swap-strong {
    color: var(--color-text);
  }
  .swap-alert {
    color: var(--color-alert);
  }
  .swap-break {
    overflow-wrap: anywhere;
  }
  .swap-more summary {
    cursor: pointer;
    color: var(--color-text-quiet);
  }
</style>
