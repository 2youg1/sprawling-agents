<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  // The line between the editor and the terminal (client/Spec.lean §7-11): an
  // APG Window Splitter that moves in whole lines of the code below it,
  // so the two regions always end on a line rather than through one. A
  // drag starts on the line and nowhere else, so selecting text in either
  // region never resizes them (roadmap §3-14).
  //
  // This file is the seat (client D95): it holds the drawn line to
  // capture a drag's pointer and the drag in progress, and draws whatever
  // `./split.look.svelte` is; the key table and the wire bag are
  // `./split.ts`'s.
</script>

<script lang="ts">
  import type { Attachment } from "svelte/attachments";

  import { say } from "../../core/lang";
  import { ui } from "../../ui";
  import { lookOf, type Grip, type SplitLook, type SplitProps } from "./split";
  import Look from "./split.look.svelte";

  const props: SplitProps = $props();

  const lang = ui().lang;

  // Neither is drawn, so neither is reactive: the line the look drew,
  // and where a drag that is still going began.
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
