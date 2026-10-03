<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  import type { Sending } from "../../core/doing";
  import { say } from "../../core/lang";
  import { STOP } from "../../core/slash";
  import { ui } from "../../ui";
  import Glyph from "../parts/glyph.svelte";
  import { pressCoin, type Face } from "./coin_face";
  import { SPELLING } from "./composer";

  interface Props {
    readonly face: Face;
    // Where a send lands: `/dispatch`, `/steer`, or a steer after the tool
    // call (client/Spec.lean §4-13), said by the send face's name.
    readonly sending: Sending;
    readonly onStop: () => void;
  }

  const { face, sending, onStop }: Props = $props();
  const { lang } = ui();
  // The faint face says why it does nothing through a description a
  // screen reader reads with the name, not through the title alone,
  // which a keyboard never shows (client/spec/Views/Workspace.lean §7-11).
  const uid = $props.id();
  const hint = `${uid}-hint`;

  const name = $derived(face === "stop" ? STOP : say($lang, SPELLING[sending]));
</script>

<!-- The send face submits the form the key stands in, so Enter in the
box and a press here are one path: `pressCoin` answers `words` and the
click lets the submit through. The stop face is a plain button. -->
<button
  type={face === "stop" ? "button" : "submit"}
  class="coin group relative block size-coin shrink-0"
  data-up={face === "stop" ? "stop" : "send"}
  aria-label={name}
  aria-disabled={face === "idle" ? "true" : undefined}
  aria-describedby={face === "idle" ? hint : undefined}
  title={face === "idle" ? say($lang, "talk_enter_hint") : undefined}
  onclick={(event) => {
    switch (pressCoin(face)) {
      case "words":
        return;
      case "cancel":
        onStop();
        return;
      case "nothing":
        event.preventDefault();
    }
  }}
>
  <span id={hint} hidden>{say($lang, "talk_enter_hint")}</span>
  <span class="coin-faces" aria-hidden="true">
    <span
      class="coin-face bg-page text-text group-aria-disabled:text-text-disabled"
      data-face="send"
    >
      <Glyph name="send" size="key" />
    </span>
    <span class="coin-face bg-page text-text" data-face="stop">
      <Glyph name="stop" size="stop" solid />
    </span>
  </span>
</button>
