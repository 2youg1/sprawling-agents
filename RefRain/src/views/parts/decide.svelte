<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  // The one card for everything that stops and asks the person
  // (client/Spec.lean §7C). This is its seat: it reads the key table and
  // hears the three chords; `decide.look.svelte` draws the card, and
  // `decide.ts` holds what an answer is and which chord reaches it.
  //
  // **An answer is a choice the caller hands in, never an action this
  // file knows.** The card decides how answers look and which key
  // reaches each; what an answer sends is the caller's, because only
  // the caller knows whether it is an approval, a dismissal or a
  // proposal's verdict.
  export type { Answer, Choice, DecideKind, DecideProps } from "./decide";
</script>

<script lang="ts">
  import type { Attachment } from "svelte/attachments";
  import { on } from "svelte/events";

  import { keymap, marks } from "../../core/keys";
  import { pressedOf } from "../../core/press";
  import Look from "./decide.look.svelte";
  import { answering, lookOf, ordered, type DecideProps } from "./decide";

  const props: DecideProps = $props();

  const keys = keymap();
  const uid = $props.id();

  // The three chords answer this card while the focus is anywhere inside
  // it, and nowhere else: they are the card's, read through the one key
  // table so a rebinding reaches them. Each holds the accelerator, so a
  // letter typed while the focus sat on the card answers nothing. The
  // keys are heard on the card itself, below every control in it.
  const hear: Attachment<HTMLDivElement> = (node) =>
    on(node, "keydown", (event) => {
      if (event.isComposing) return;
      const chosen = answering(ordered(props.choices), keys.acting(pressedOf(event)));
      if (chosen === undefined) return;
      event.preventDefault();
      event.stopPropagation();
      chosen.onPress();
    });

  const look = $derived(
    lookOf(props, {
      uid,
      hear,
      marksOf: (action) => marks(keys.chord(action), keys.platform),
    }),
  );
</script>

<Look {...look} body={props.body} />
