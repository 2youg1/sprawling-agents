<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  import type { TomlLook } from "./toml";

  const { region, toggle, path, text, unread }: TomlLook = $props();
</script>

<aside {...region} class="min-w-0">
  <!-- The summary keeps the platform's disclosure mark, which a flex
  summary would drop, and brightens under the pointer with the page's
  own duration and curve. -->
  <details class="rounded-card bg-chrome/60 px-base py-tight">
    <summary
      class="cursor-pointer text-label text-text-quiet transition-colors duration-short ease-leave hover:text-text hover:ease-arrive"
    >
      {toggle}
      <code class="ml-snug font-mono text-note text-text-faint">{path}</code>
    </summary>
    {#if text !== undefined}
      <pre class="overflow-x-auto py-snug font-mono text-note text-text-quiet">{text}</pre>
    {:else}
      <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression -->
      {@render unread()}
    {/if}
  </details>
</aside>
