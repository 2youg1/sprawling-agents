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
  // One hint, drawn beside the control it is about. The seat: it owns
  // the id, the dismissal and the listeners; `tip.look.svelte` draws.
  //
  // This replaces `title`, which fails three readers: it waits about a
  // second for a pointer a keyboard never has, no key reaches it, and a
  // touch screen never draws it at all. The hint here appears on hover
  // and on focus anywhere inside the wrapper, so tabbing to the control
  // shows it.
  //
  // **The caller states the relation, because only the caller knows
  // whether the control has a name already.** The child is handed the
  // hint's id and writes `aria-describedby` when it is named by its own
  // text, or `aria-labelledby` when these words are the only name it
  // has. Writing neither leaves a hint a screen reader never reads. A
  // part whose wire bag already names the hint passes that id in.
  //
  // **Placement has two branches, because engines disagree.** Where
  // anchor positioning is implemented the hint is `fixed` against the
  // anchor and leaves every box that clips behind it. Where it is not -
  // Safari and Firefox today - the hint is placed against the wrapper,
  // which is why the wrapper is `relative`. Shipping only the first
  // branch fails silently: the hint keeps a static position and can land
  // outside the window.
  //
  // **Escape dismisses the hint and nothing else** (client/Spec.lean §7-3,
  // WCAG 1.4.13). It stays away while the pointer or the focus stays,
  // and the next re-engagement summons it back. The dismissal listens
  // at the window because a hint raised by hover has no focused element
  // to hear the key; the two re-engagement listeners ride an attachment
  // on the wrapper for the same reason the rest of this does not: a
  // wrapper is not a control and must not grow an interactive role just
  // to hold two listeners.
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
