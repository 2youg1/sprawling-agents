<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  import type { Snippet } from "svelte";

  import type { ModalWire, Seat } from "./modal";

  interface Props {
    readonly wire: ModalWire;
    readonly seat: Seat;
    readonly children: Snippet;
  }

  const { wire, seat, children }: Props = $props();
</script>

<dialog
  {...wire}
  class={[
    "m-auto mt-section hidden w-full max-w-measure rounded-panel bg-raised p-0 opacity-0 shadow-sheet",
    "transition-[opacity,display,overlay] transition-discrete duration-panel ease-leave",
    "open:block open:opacity-100 open:ease-arrive starting:open:opacity-0 still:transition-none",
    "backdrop:bg-transparent backdrop:backdrop-brightness-50",
    seat === "specimen" ? "static" : "",
  ]}
>
  <div class="flex flex-col gap-snug p-snug">
    {@render children()}
  </div>
</dialog>
