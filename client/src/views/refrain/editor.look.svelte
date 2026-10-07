<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // How RefRain's editor is dressed (client/Spec.lean §7N, D23): the box
  // a CodeMirror view is mounted into, filling the column it stands in,
  // and the rules that overrule CodeMirror's own base theme so every
  // colour on it is a role of the theme. CodeMirror builds its DOM
  // itself, outside this template, so those rules reach it through
  // `:global()` under this box's own class and touch no other editor.
  // Inserted text takes the accent's pale ground and removed text the
  // alert's, as the diff does on docs/frontend-method.md §7F; the
  // cursor's line carries the 2 px accent bar. Every editor the client
  // opens - the source and diff readings, the versions' comparison, a
  // format's comparison, the colour page's stylesheet - is mounted in
  // this box, so they all wear the same dress.
  import type { EditorLook } from "./editor";

  const look: EditorLook = $props();
</script>

<div {...look.wire} class="editor"></div>

<style>
  .editor {
    flex: 1 1 auto;
    min-height: 0;
  }
  .editor :global(.cm-editor) {
    height: 100%;
    background: var(--color-page);
    color: var(--color-text);
  }
  .editor :global(.cm-editor.cm-focused) {
    outline: none;
  }
  .editor :global(.cm-scroller) {
    font-family: var(--font-mono);
    font-size: var(--text-note);
    line-height: calc(var(--spacing-baseline) * 3);
  }
  .editor :global(.cm-content) {
    padding: var(--spacing-snug) 0;
    caret-color: var(--color-accent);
  }
  .editor :global(.cm-cursor) {
    border-left-color: var(--color-accent);
  }
  .editor :global(.cm-gutters) {
    background: var(--color-page);
    color: var(--color-text-faint);
    border: none;
  }
  .editor :global(.cm-lineNumbers .cm-gutterElement) {
    padding: 0 var(--spacing-base) 0 var(--spacing-wide);
  }
  .editor :global(.cm-activeLine) {
    background: transparent;
    box-shadow: inset var(--spacing-hair) 0 0 var(--color-accent);
  }
  .editor :global(.cm-activeLineGutter) {
    background: transparent;
    color: var(--color-text-quiet);
  }
  .editor :global(.cm-selectionBackground),
  .editor :global(.cm-content ::selection) {
    background: color-mix(in oklch, var(--color-accent) 28%, transparent);
  }
  .editor :global(.cm-searchMatch) {
    background: color-mix(in oklch, var(--color-accent) 18%, transparent);
    outline: 1px solid color-mix(in oklch, var(--color-accent) 50%, transparent);
  }
  .editor :global(.cm-searchMatch-selected) {
    background: color-mix(in oklch, var(--color-accent) 36%, transparent);
  }
  .editor :global(.cm-specialChar) {
    color: var(--color-alert);
  }
  .editor :global(.cm-panels) {
    background: var(--color-chrome);
    color: var(--color-text-quiet);
    border-color: var(--color-edge);
  }
  .editor :global(.cm-panel.cm-search) {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--spacing-tight) var(--spacing-snug);
    padding: var(--spacing-snug) var(--spacing-wide);
    font-family: var(--font-mono);
    font-size: var(--text-note);
  }
  .editor :global(.cm-panel.cm-search br) {
    flex-basis: 100%;
  }
  .editor :global(.cm-textfield) {
    height: var(--spacing-control-sm);
    padding: 0 var(--spacing-snug);
    border: 1px solid var(--color-edge-input);
    border-radius: var(--radius-control);
    background: var(--color-page);
    color: var(--color-text);
    font: inherit;
  }
  /* A search panel key rests on the raised ground and rises a step under
     the hand, as every control does; it leaves with the leaving curve
     and arrives with the arriving one (docs/frontend-method.md §4-43). */
  .editor :global(.cm-button) {
    height: var(--spacing-control-sm);
    padding: 0 var(--spacing-snug);
    border: none;
    border-radius: var(--radius-control);
    background: var(--color-raised);
    background-image: none;
    color: var(--color-text);
    font: inherit;
    transition: background-color var(--transition-duration-short) var(--ease-leave);
  }
  .editor :global(.cm-button:hover) {
    background: var(--color-raised-hover);
    transition-timing-function: var(--ease-arrive);
  }
  .editor :global(.cm-panel.cm-search label) {
    display: inline-flex;
    align-items: center;
    gap: var(--spacing-tight);
  }
  .editor :global(.cm-panel.cm-search [name="close"]) {
    margin-inline-start: auto;
    color: var(--color-text-faint);
  }
  .editor :global(.cm-editor.cm-merge-b .cm-changedLine) {
    background: color-mix(in oklch, var(--color-accent) 10%, transparent);
  }
  .editor :global(.cm-editor.cm-merge-b .cm-changedText) {
    background: color-mix(in oklch, var(--color-accent) 26%, transparent);
  }
  .editor :global(.cm-editor .cm-deletedChunk) {
    background: color-mix(in oklch, var(--color-alert) 10%, transparent);
  }
  .editor :global(.cm-editor .cm-deletedChunk .cm-deletedText),
  .editor :global(.cm-editor.cm-merge-b .cm-deletedText) {
    background: color-mix(in oklch, var(--color-alert) 26%, transparent);
  }
  .editor :global(.cm-editor.cm-merge-b .cm-changedLineGutter) {
    background: var(--color-accent);
  }
  .editor :global(.cm-editor .cm-deletedLineGutter) {
    background: var(--color-alert);
  }
  @media (forced-colors: active) {
    .editor :global(.cm-editor.cm-merge-b .cm-changedLine),
    .editor :global(.cm-editor .cm-deletedChunk) {
      border-inline-start: var(--spacing-hair) solid CanvasText;
    }
  }
</style>
