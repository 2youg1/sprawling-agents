<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  // How the stack of refusals is drawn, and nothing else
  // (`./refusal.ts`, `RefusalLook`): a column of toasts, each the
  // notice the seat draws in it. On the composer the stack stands on the
  // composer's upper edge, clear of it, so it never covers the coin key,
  // which is the retry; at a page's foot it is centred, a measure wide
  // at most. A toast arrives on the arriving curve and fades out on the
  // leaving one before its box is hidden (`allow-discrete`), and stands
  // still when the person or the machine asks for less motion
  // (docs/frontend-method.md §4-43).
  import type { RefusalLook } from "./refusal";

  const look: RefusalLook = $props();
  const placed = $derived(look.stand.kind === "composer" ? look.stand : undefined);
</script>

<ul
  class={["stack flex flex-col items-stretch gap-snug", look.stand.kind]}
  style:left={placed === undefined ? undefined : `${String(placed.left)}px`}
  style:width={placed === undefined ? undefined : `${String(placed.width)}px`}
  style:bottom={placed === undefined ? undefined : `${String(placed.bottom)}px`}
>
  {#each look.toasts as toast (toast.key)}
    <li
      {...toast.wire}
      class={[
        "transition-[opacity,display] transition-discrete duration-panel still:transition-none",
        toast.gone ? "hidden opacity-0 ease-leave" : "opacity-100 ease-arrive",
      ]}
    >
      {@render look.body(toast.key)}
    </li>
  {/each}
</ul>

<style>
  .composer,
  .foot {
    position: fixed;
  }

  .composer {
    padding-bottom: var(--spacing-snug);
  }

  /* Where the composer would stand on a page that has none: above the
   * frame's margin by the height of an eight-line box and its two
   * section gaps. */
  .foot {
    bottom: calc(var(--spacing-margin) + 8 * var(--spacing-baseline) + var(--spacing-section) * 2);
    left: 50%;
    width: min(var(--container-measure), calc(100vw - 2 * var(--spacing-margin)));
    translate: -50% 0;
  }
</style>
