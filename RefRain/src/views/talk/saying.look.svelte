<!-- This Source Code Form is subject to the terms of the Mozilla Public
License, v. 2.0. If a copy of the MPL was not distributed with this
file, You can obtain one at https://mozilla.org/MPL/2.0/.
Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  import type { SayingLook } from "./saying";
  const { who, laid, settled, edge }: SayingLook = $props();
</script>

<div class="text-body">
  <div class="mb-tight text-note text-text-faint">{who}</div>
  {@render laid()}
  <div class="whitespace-pre-wrap break-words font-read leading-read">
    {settled}<span class="text-text-faint">{edge}</span><span class="caret blink bg-accent"></span>
  </div>
</div>

<style>
  .caret {
    display: inline-block;
    width: var(--spacing-caret);
    height: var(--spacing-caret);
    margin-inline-start: var(--spacing-tight);
    vertical-align: baseline;
  }
</style>
