<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  export type { TipSide } from "./tip";
</script>

<script lang="ts">
  
  
  import type { Attachment } from "svelte/attachments";
  import { on } from "svelte/events";

  import Look from "./tip.look.svelte";
  import { dismisses, lookOf, type TipProps } from "./tip";

  const props: TipProps = $props();
  const uid = $props.id();

  let dismissed = $state(false);

  const reengage: Attachment<HTMLSpanElement> = (node) => {
    const enter = on(node, "mouseenter", () => {
      dismissed = false;
    });
    const focus = on(node, "focusin", () => {
      dismissed = false;
    });
    return () => {
      enter();
      focus();
    };
  };

  function heard(event: KeyboardEvent): void {
    if (dismisses(event.key)) dismissed = true;
  }

  const look = $derived(lookOf(props, { uid, dismissed, reengage }));
</script>

<svelte:window onkeydown={heard} />

<Look {...look} children={props.children} />
