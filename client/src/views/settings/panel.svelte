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
  // A click on the backdrop lands on the `<dialog>` itself, because the
  // inner frame fills the whole element: that is the one click that
  // closes it.

  import type { SetupGroup, View } from "../../core/route";
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

  // Opening hands the focus to the entry that names the group drawn,
  // rather than to the first control the platform would pick.
  $effect(() => {
    if (sheet === undefined || sheet.open) return;
    sheet.showModal();
    sheet.querySelector<HTMLElement>("nav [aria-current]")?.focus();
  });
</script>

<!-- One width: from the window's left edge to the shell's right silver
  line (client D24), so its tree stands where the sessions pane stands and its
  group where the conversation stands; the whole window on one column.
  The arrival from the left is `.settings-panel` in `theme/settings.css`, a
  transition from its `@starting-style`. -->
<dialog
  bind:this={sheet}
  class="settings-panel fixed inset-y-0 left-0 m-0 h-dvh max-h-none w-full max-w-[calc(var(--silver-side)*(1+var(--silver))-var(--spacing-gutter)/2)] bg-page narrow:max-w-none p-0 text-body text-text shadow-sheet backdrop:bg-transparent backdrop:backdrop-brightness-50"
  aria-labelledby={`${uid}-title`}
  oncancel={(event) => {
    // Escape asks the caller, which moves the address bar; closing the
    // element here would leave the address saying it is open.
    event.preventDefault();
    onClose();
  }}
  onclick={(event) => {
    if (event.target === sheet) onClose();
  }}
>
  <Sheet {group} {beneath} {onPick} {onClose} titleId={`${uid}-title`} />
</dialog>
