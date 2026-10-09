<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  // The file finder: find a file of the building in front by a part of its name
  // (refrain 3-11, client/Spec.lean §4-62). What it holds is
  // `finder/search.svelte`; this file is the modal around it, whose
  // wiring is `modal.ts`'s and whose drawing is `modal.look.svelte`'s.
  // A click on the backdrop closes nothing: the box holds a half-typed
  // name, and a stray click should not throw it away.

  import type { Address } from "../wire";
  import Search from "./finder/search.svelte";
  import { modalHold, modalWire } from "./modal";
  import Modal from "./modal.look.svelte";

  interface Props {
    readonly under: Address;
    readonly onClose: () => void;
  }

  const { under, onClose }: Props = $props();

  const uid = $props.id();
  const wire = modalWire({
    seat: "modal",
    named: { "aria-labelledby": `${uid}-title` },
    backdrop: "stays",
    hold: modalHold("modal"),
    onClose: () => {
      onClose();
    },
  });
</script>

<Modal {wire} seat="modal">
  <Search {under} titleId="{uid}-title" {onClose} />
</Modal>
