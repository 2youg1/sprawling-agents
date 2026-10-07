<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->
<script lang="ts">
  // How the microphone is drawn, and nothing else: a pill that reads
  // quiet while it waits, turns the alert colour while it records and
  // goes faint while the city transcribes (`./record.ts`, `RecordLook`).
  // Its hit area reaches past the pill, because the pill is shorter than
  // a fingertip.
  import type { RecordLook } from "./record";

  const look: RecordLook = $props();
</script>

<button class="microphone" data-hearing={look.hearing} {...look.wire}>{look.label}</button>
{#if look.refused !== undefined}
  <span class="refused">{look.refused}</span>
{/if}

<style>
  .microphone {
    position: relative;
    display: flex;
    flex-shrink: 0;
    align-items: center;
    height: var(--spacing-control-sm);
    padding-inline: var(--spacing-base);
    border-radius: var(--radius-pill);
    background-color: var(--color-raised);
    color: var(--color-text-quiet);
    font-size: var(--text-note);
    transition:
      background-color var(--transition-duration-short) var(--ease-leave),
      color var(--transition-duration-short) var(--ease-leave);
  }
  .microphone::before {
    content: "";
    position: absolute;
    inset: calc(var(--spacing-snug) * -1);
  }
  .microphone[data-hearing="ready"]:hover {
    background-color: var(--color-raised-hover);
    color: var(--color-text);
    transition-timing-function: var(--ease-arrive);
  }
  .microphone[data-hearing="taking"] {
    background-color: var(--color-alert);
    color: var(--color-on-accent);
    transition-timing-function: var(--ease-arrive);
  }
  .microphone[data-hearing="hearing"] {
    color: var(--color-text-disabled);
  }
  .refused {
    color: var(--color-alert);
    font-size: var(--text-note);
  }
</style>
