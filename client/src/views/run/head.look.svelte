<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<!-- How the run page's sheet is drawn, and nothing else (`./head`,
`HeadLook`): each fact a cell of one grid, its name small above its
value, the way the panorama's session sheet sets them. A link lights on
the arriving curve and settles on the leaving one
(docs/frontend-method.md §4-43). -->
<script lang="ts">
  import type { Face, HeadLook, LineLook } from "./head";

  const look: HeadLook = $props();

  const FACE: Record<Face, string> = {
    words: "",
    figure: "figure",
    mono: "font-mono",
  };
</script>

{#snippet line(each: LineLook)}
  {#if each.kind === "text"}
    <dd class={["truncate", FACE[each.face], each.rank === "value" ? "text-text" : "text-note text-text-quiet"]}>
      {each.text}
    </dd>
  {:else if each.kind === "time"}
    <dd class="figure truncate text-note text-text-quiet"><time datetime={each.at}>{each.text}</time></dd>
  {:else}
    <dd class="truncate font-mono">
      <a
        {...each.building.wire}
        class="text-text-quiet transition-colors ease-leave still:transition-none hover:text-text hover:ease-arrive"
        >{each.building.text}</a
      ><span class="text-text-faint">/</span><a {...each.room.wire} class="text-text">{each.room.text}</a>
    </dd>
  {/if}
{/snippet}

<dl
  class="grid grid-cols-[repeat(auto-fill,minmax(22ch,1fr))] gap-x-gutter gap-y-base border-y border-edge py-base"
  aria-label={look.label}
>
  {#each look.cells as cell (cell.key)}
    <div class={["flex min-w-0 flex-col", cell.wide && "col-span-2"]}>
      <dt class="text-note text-text-faint">{cell.name}</dt>
      {#each cell.lines as each (each.key)}
        <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression (a snippet call is the render itself; the typechecker types local snippet calls as returning void) -->
        {@render line(each)}
      {/each}
    </div>
  {/each}
</dl>
