<!-- This Source Code Form is subject to the terms of the Mozilla Public
License, v. 2.0. If a copy of the MPL was not distributed with this
file, You can obtain one at https://mozilla.org/MPL/2.0/.
Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  import type { HeadLook } from "./head";
  const { who, model, time, ttft, tps, rhythm }: HeadLook = $props();
</script>

<div class="min-w-0 text-note text-text-faint">
  <span class="me-tight font-label text-label whitespace-nowrap text-text">{who}</span>
  {#if model !== null}
    <span class="me-tight wrap-anywhere">{model}</span>
  {/if}
  <time class="figure me-tight whitespace-nowrap" datetime={time.iso}>{time.text}</time>
  {#if ttft !== undefined}
    <span class="figure me-tight whitespace-nowrap">{ttft}</span>
  {/if}
  {#if tps !== undefined}
    <span class="figure me-tight whitespace-nowrap">{tps}</span>
  {/if}
  {@render rhythm?.()}
</div>
