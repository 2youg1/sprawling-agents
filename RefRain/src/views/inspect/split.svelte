<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  
  
</script>

<script lang="ts">
  import type { Attachment } from "svelte/attachments";

  import { say } from "../../core/lang";
  import { ui } from "../../ui";
  import { lookOf, type Grip, type SplitLook, type SplitProps } from "./split";
  import Look from "./split.look.svelte";

  const props: SplitProps = $props();

  const lang = ui().lang;

  
  let line: HTMLElement | undefined;
  const grip: { from: Grip | null } = { from: null };
  const hold: Attachment<HTMLElement> = (node) => {
    line = node;
    return () => {
      if (line === node) line = undefined;
    };
  };

  const look: SplitLook = $derived(
    lookOf(props, say($lang, "inspect_split"), {
      capture: (pointer) => {
        line?.setPointerCapture(pointer);
      },
      hold,
      grip,
    }),
  );
</script>

<Look {...look} />
