<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  import type { CopyLook } from "../../../src/views/parts/copy";

  const look: CopyLook = $props();
  const uid = $props.id();
</script>

<span class="copy">
  <button
    {...look.key}
    class={["key", look.refused ? "refused" : ""]}
    aria-describedby={`${uid}-note`}
  >
    {look.form === "named" ? look.name : look.face === "check" ? "✓" : look.refused ? "×" : "⧉"}
  </button>
  <span id={`${uid}-note`} class="note">{look.note}</span>
  <span class="heard" {...look.heard}>{look.said}</span>
</span>

<style>
  .copy {
    display: inline-flex;
    align-items: baseline;
    gap: var(--spacing-tight);
    min-width: 0;
    max-width: 50%;
  }
  .key {
    flex-shrink: 0;
    min-height: var(--spacing-control-sm);
    padding: 0 var(--spacing-snug);
    border: 1px solid var(--color-edge-input);
    border-radius: var(--radius-control);
    background: transparent;
    color: var(--color-text-quiet);
    font: inherit;
    transition: border-color var(--transition-duration-short) var(--ease-leave);
  }
  .key:hover {
    border-color: var(--color-accent);
    transition-timing-function: var(--ease-arrive);
  }
  .refused {
    color: var(--color-alert);
  }
  .note {
    min-width: 0;
    overflow: hidden;
    color: var(--color-text-faint);
    font-size: var(--text-note);
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .heard {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip-path: inset(50%);
  }
</style>
