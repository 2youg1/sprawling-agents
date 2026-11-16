<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
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

  .foot {
    bottom: calc(var(--spacing-margin) + 8 * var(--spacing-baseline) + var(--spacing-section) * 2);
    left: 50%;
    width: min(var(--container-measure), calc(100vw - 2 * var(--spacing-margin)));
    translate: -50% 0;
  }
</style>
