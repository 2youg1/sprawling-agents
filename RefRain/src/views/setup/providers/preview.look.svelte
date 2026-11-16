<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  import type { ReadingLook } from "./preview";

  const { readings }: { readonly readings: readonly ReadingLook[] } = $props();
</script>

{#each readings as reading (reading.key)}
  <details class="rounded-card bg-chrome px-base py-snug text-note">
    <summary
      class="w-fit cursor-pointer rounded-control px-tight text-text-quiet transition-[background-color] ease-leave hover:bg-raised hover:ease-arrive"
      >{reading.title}</summary
    >
    <pre
      class={[
        "mt-snug overflow-auto font-mono text-text-faint",
        reading.wraps && "whitespace-pre-wrap",
      ]}>{reading.text}</pre>
  </details>
{/each}
