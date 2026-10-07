<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // One chord, drawn: one glyph per `<kbd>`, no focus and no keys of its
  // own. Quiet ink, because a chord beside a control is a reminder, not
  // the control.
  import type { KbdLook } from "./kbd";

  const { marks, class: placed }: KbdLook = $props();
</script>

<span class={["inline-flex items-center gap-tight", placed]}>
  {#each marks as mark (mark)}
    <kbd class="rounded-control px-tight font-mono text-note leading-none text-text-quiet no-underline">{mark}</kbd>
  {/each}
</span>
