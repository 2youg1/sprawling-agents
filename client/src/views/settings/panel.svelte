<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  // The settings panel: a native modal `<dialog>` that arrives from the
  // left edge, the settings tree on its left and one group drawn beside
  // it (client/Spec.lean §7L). The platform owns the modality the way
  // `parts/dialog.svelte` lets it (4-20): `showModal()` gives the top
  // layer, the focus trap, the rest of the page `inert` and Escape, so
  // this file holds no scrim, no keydown for Escape and no z-index.
  //
  // **Open is an address.** The shell mounts this while the address bar
  // says `#/setup[/<group>]` and takes it away when it says anything
  // else, so a reload, the back button and a link all agree with what
  // is on screen; picking a group replaces the address rather than
  // pushing one, so the back button leaves the panel instead of walking
  // back through every group looked at.
  //
  // This file is the seat: it owns the element and when it opens; the
  // look (`panel.look.svelte`) draws the frame and its arrival, and
  // spreads the wiring `dialogOf` builds (`panel.ts`).

  import type { SetupGroup, View } from "../../core/route";
  import { dialogOf } from "./panel";
  import Look from "./panel.look.svelte";
  import Sheet from "./sheet.svelte";

  interface Props {
    readonly group: SetupGroup;
    readonly beneath: View;
    readonly onPick: (group: SetupGroup) => void;
    readonly onClose: () => void;
  }

  const { group, beneath, onPick, onClose }: Props = $props();

  const uid = $props.id();

  let sheet = $state<HTMLDialogElement | undefined>(undefined);
  const hold = (node: HTMLDialogElement): (() => void) => {
    sheet = node;
    return () => {
      sheet = undefined;
    };
  };

  // Opening hands the focus to the entry that names the group drawn,
  // rather than to the first control the platform would pick.
  $effect(() => {
    if (sheet === undefined || sheet.open) return;
    sheet.showModal();
    sheet.querySelector<HTMLElement>("nav [aria-current]")?.focus();
  });
</script>

<Look dialog={dialogOf(`${uid}-title`, onClose, hold)}>
  <Sheet {group} {beneath} {onPick} {onClose} titleId={`${uid}-title`} />
</Look>
