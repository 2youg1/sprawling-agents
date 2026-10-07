<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // A draft the browser would not store (refrain §3-14, the second row;
  // client/Spec.lean §4-63): one line that says the words live in this
  // tab alone and are gone when it closes, and a copy beside it, so the
  // person can carry them out before that. The box and RefRain draw it
  // under what they hold. This is the seat; `./unkept.look.svelte`
  // draws it, and the copy key is the one copy key (`./copy`).
  import { say } from "../../core/lang";
  import { ui } from "../../ui";
  import Look from "./unkept.look.svelte";

  interface Props {
    // The words as they stand when the copy is pressed.
    readonly words: () => string;
  }

  const { words }: Props = $props();

  const lang = ui().lang;
</script>

<Look
  sentence={say($lang, "part_unkept")}
  line={{ role: "status" }}
  copy={{ text: words, note: say($lang, "unkept_copy_note") }}
/>
