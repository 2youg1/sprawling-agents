<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  // The question asked before something that cannot be taken back. An
  // action that can be undone is done straight away and offered back as
  // a Notice; only the irreversible one stops and explains first.
  //
  // **The platform owns the modality.** `showModal()` puts the element
  // in the top layer, traps the focus, marks the rest of the page
  // `inert`, and answers Escape - four behaviours this file used to
  // carry as a wrapper, a keydown handler, a stacking order and a focus
  // call. The top layer also means no `z-index`: nothing on the page can
  // be given a number that puts it over a modal dialog.
  //
  // The seat holds the element and carries `open` to it; what is drawn
  // is `dialog.look.svelte`, and what the look is handed is `dialog.ts`.
  export type { DialogProps } from "./dialog";
</script>

<script lang="ts">
  import type { Attachment } from "svelte/attachments";

  import Look from "./dialog.look.svelte";
  import { lookOf, type DialogProps } from "./dialog";

  const props: DialogProps = $props();

  // One instance, one id root: the title and the detail derive their
  // ids from it, so the two ARIA references name elements that are
  // unique per instance and stable across hydration.
  const uid = $props.id();

  // The element is held as state rather than plain storage so that the
  // effect below runs again when the ref arrives, whichever order the
  // first render and the first `open` happen in.
  let sheet = $state<HTMLDialogElement | undefined>(undefined);
  const hold: Attachment<HTMLDialogElement> = (node) => {
    sheet = node;
    return () => {
      sheet = undefined;
    };
  };

  // The caller owns whether the question stands; this only carries that
  // answer to the element. Asking an already-open dialog to open throws,
  // so each call is made only on the edge it belongs to.
  $effect(() => {
    if (sheet === undefined) return;
    if (props.open) {
      if (!sheet.open) sheet.showModal();
      return;
    }
    if (sheet.open) sheet.close();
  });

  const look = $derived(lookOf(props, uid, hold));
</script>

<Look {...look} />
