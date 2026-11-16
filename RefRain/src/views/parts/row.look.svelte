<script lang="ts">
  // This Source Code Form is subject to the terms of the Mozilla Public
  // License, v. 2.0. If a copy of the MPL was not distributed with this
  // file, You can obtain one at https://mozilla.org/MPL/2.0/.
  // Copyright (c) 2026 2youg1 and the sprawling contributors

  import type { RowLook } from "./row";

  const look: RowLook = $props();
</script>

{#snippet texts()}
  <span class="truncate text-body text-text">{look.primary}</span>
  {#if look.secondary !== undefined}
    <span class="summary truncate text-note text-text-faint">{look.secondary}</span>
  {/if}
{/snippet}

<li
  {...look.item}
  class={[
    "flex min-h-touch w-full min-w-0 items-center gap-base border-b border-edge px-base py-snug",
    "transition-[background-color] ease-leave hover:wash hover:ease-arrive has-[:focus-visible]:wash has-[:focus-visible]:ease-arrive",
    look.chosen && "chosen",
  ]}
>
  {#if look.open !== undefined}
    <button {...look.open} class="flex min-w-0 flex-1 flex-col text-left">
      <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression (a snippet call is the render itself; the typechecker types local snippet calls as returning void) -->
      {@render texts()}
    </button>
  {:else}
    <div class="flex min-w-0 flex-1 flex-col text-left">
      <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression (a snippet call is the render itself; the typechecker types local snippet calls as returning void) -->
      {@render texts()}
    </div>
  {/if}
  {#if look.status !== undefined}
    <div class="shrink-0">{@render look.status()}</div>
  {/if}
  {#if look.actions !== undefined}
    <div class="flex shrink-0 items-center gap-tight">{@render look.actions()}</div>
  {/if}
</li>
