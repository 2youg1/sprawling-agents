<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  // How the file finder is drawn, and nothing else (`./search.ts`,
  // `SearchLook`): the title, the box sunk one step into the raised
  // sheet, and the paths as menu entries - a file's name in mono, then
  // its directory, quieter. An entry rests on the sheet and lifts one
  // step under the pointer and under the cursor (docs/frontend-method.md
  // §7A-5), arriving and leaving on the theme's two curves (§4-43).
  import type { SearchLook } from "./search";

  const look: SearchLook = $props();
</script>

<h2 {...look.title.wire} class="px-snug text-label text-text-quiet">{look.title.text}</h2>
<input
  {...look.box}
  class="w-full rounded-control bg-page px-base py-snug text-body placeholder:text-text-faint"
/>
<ul {...look.list} class="max-h-palette overflow-y-auto">
  {#each look.options as option (option.key)}
    <li
      {...option.wire}
      class={[
        "flex cursor-pointer items-baseline gap-snug rounded-control px-base py-snug font-mono text-body",
        "transition-colors ease-leave hover:bg-raised-hover hover:ease-arrive",
        option.active ? "bg-raised-hover text-text ease-arrive" : "text-text-quiet",
      ]}
    >
      <span class="truncate">{option.name}</span>
      <span class="min-w-0 truncate text-note text-text-faint">{option.dir}</span>
    </li>
  {/each}
</ul>
{#each look.notes as note (note)}
  <p class="px-snug text-note text-text-faint">{note}</p>
{/each}
