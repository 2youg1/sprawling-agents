<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // A second look for the irreversible question, written the way a
  // component library writes one: native elements, no utility classes,
  // every style in its own block and read from the theme's tokens. It
  // takes the same `DialogLook` as the shipped look and spreads the same
  // wire bag on its `<dialog>`, so drawing the question with it proves
  // that the modality, the relations, Escape and the order of the two
  // answers live in `dialog.ts` and the seat, not in the shipped markup.
  import type { DialogLook } from "../../../src/views/parts/dialog";

  const { sheet, stands, heading, detail, grounds, answers }: DialogLook = $props();
</script>

<dialog {...sheet} class="question" class:in-flow={stands === "in-flow"}>
  <h2 id={heading.id}>{heading.text}</h2>
  {#if detail !== undefined}
    <p id={detail.id}>{detail.text}</p>
  {/if}
  {#if grounds !== undefined}{@render grounds()}{/if}
  <div class="answers">
    {#each answers as answer (answer.tone)}
      <button type="button" class={answer.tone} onclick={answer.onPress}>{answer.label}</button>
    {/each}
  </div>
</dialog>

<style>
  .question {
    margin: auto;
    width: 100%;
    max-width: var(--container-measure);
    border: 1px solid var(--color-edge-panel);
    border-radius: var(--radius-card);
    background: var(--color-raised);
    color: var(--color-text);
    padding: var(--spacing-wide);
    opacity: 0;
    transition:
      opacity var(--transition-duration-panel) var(--ease-leave),
      display var(--transition-duration-panel) allow-discrete,
      overlay var(--transition-duration-panel) allow-discrete;
  }

  .question[open] {
    display: grid;
    gap: var(--spacing-snug);
    opacity: 1;
    transition-timing-function: var(--ease-arrive);
  }

  @starting-style {
    .question[open] {
      opacity: 0;
    }
  }

  .question.in-flow {
    position: static;
  }

  .question::backdrop {
    backdrop-filter: brightness(0.5);
  }

  h2 {
    margin: 0;
    font-size: var(--text-heading);
  }

  p {
    margin: 0;
    font-size: var(--text-note);
    color: var(--color-text-quiet);
  }

  .answers {
    display: flex;
    flex-direction: row-reverse;
    justify-content: flex-start;
    gap: var(--spacing-snug);
  }

  button {
    border: 1px solid var(--color-edge-input);
    border-radius: var(--radius-control);
    background: transparent;
    color: var(--color-text);
    padding: var(--spacing-tight) var(--spacing-base);
    font-size: var(--text-label);
  }

  button.primary {
    border-color: var(--color-accent);
    color: var(--color-accent);
  }

  button.destructive {
    border-color: var(--color-alert);
    color: var(--color-alert);
  }
</style>
