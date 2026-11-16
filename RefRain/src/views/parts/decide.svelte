<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  
  
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
