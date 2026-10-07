<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  // How the palette and the file finder are drawn around what they
  // hold, and nothing else: the key sheet's box (`parts/kbd.svelte`) - a
  // section below the top edge, centred, a shadowed face with no border
  // over a page dimmed behind it. It fades in through `@starting-style`
  // on the arriving curve and out through `allow-discrete` on the
  // leaving one, moves only opacity, and stands still when the person or
  // the machine asks for less motion (docs/frontend-method.md §4-43).
  //
  // The padding sits on the inner box, which fills the element, so a
  // click that lands on the element itself is a click on its backdrop.
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
