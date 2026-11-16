<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  import type { Snippet } from "svelte";

  const TALK = 760;

  const FRAME = "min-w-0 rounded-card border border-dashed border-edge-input p-snug";

  interface CaseProps {
    readonly label: string;
    readonly width?: number;
    readonly children: Snippet;
  }
</script>

<script lang="ts">
  const { label, width, children }: CaseProps = $props();
</script>

<section class="mb-wide" aria-label={label}>
  <div class="mb-tight text-note text-text-faint">{label}</div>
  {#if width === undefined}
    <div class="w-full {FRAME}" style:max-width="{String(TALK)}px">{@render children()}</div>
  {:else}
    <div class="w-max max-w-full {FRAME}">
      <div class="@container" style:width="{String(width)}px" style:max-width="100%">
        {@render children()}
      </div>
    </div>
  {/if}
</section>
