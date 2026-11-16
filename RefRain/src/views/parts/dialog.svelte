<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  export type { DialogProps } from "./dialog";
</script>

<script lang="ts">
  import type { Attachment } from "svelte/attachments";

  import Look from "./dialog.look.svelte";
  import { lookOf, type DialogProps } from "./dialog";

  const props: DialogProps = $props();

  const uid = $props.id();

  let sheet = $state<HTMLDialogElement | undefined>(undefined);
  const hold: Attachment<HTMLDialogElement> = (node) => {
    sheet = node;
    return () => {
      sheet = undefined;
    };
  };

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
