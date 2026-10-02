<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  // Accel-P: find a file of the building in front by a part of its name
  // (refrain 3-11, client/Spec.lean §4-62). What it holds is
  // `finder/search.svelte`; this file is the modal around it.
  //
  // **A modal dialog the platform owns**, as the key sheet is: the top
  // layer, the trapped focus, the inert page behind and the focus given
  // back on close are the platform's, so there is no `z-index` and no
  // focus code here. Escape arrives as a cancel request, answered by the
  // caller rather than by the element.

  import type { Address } from "../wire";
  import Search from "./finder/search.svelte";

  interface Props {
    readonly under: Address;
    readonly onClose: () => void;
  }

  const { under, onClose }: Props = $props();

  // The sheet's box, in the shape the key sheet draws (`parts/kbd.svelte`).
  const SHEET =
    "m-auto mt-section hidden w-full max-w-measure flex-col gap-snug rounded-panel " +
    "bg-raised p-snug opacity-0 shadow-sheet transition-[opacity,display,overlay] " +
    "transition-discrete duration-panel ease-leave open:flex open:opacity-100 open:ease-arrive " +
    "starting:open:opacity-0 motion-reduce:transition-none " +
    "backdrop:bg-transparent backdrop:backdrop-brightness-50";

  const uid = $props.id();
  let sheet = $state<HTMLDialogElement | undefined>(undefined);

  $effect(() => {
    const node = sheet;
    if (node === undefined) return;
    node.showModal();
    return () => {
      node.close();
    };
  });
</script>

<dialog
  bind:this={sheet}
  class={SHEET}
  aria-labelledby="{uid}-title"
  oncancel={(event) => {
    event.preventDefault();
    onClose();
  }}
>
  <Search {under} titleId="{uid}-title" {onClose} />
</dialog>
