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
  import type { Face } from "./coin_face";
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

  const name = $derived(face === "stop" ? STOP : say($lang, SPELLING[sending]));
</script>

<!-- The send face submits the form the key stands in, so Enter in the
box and a press here are one path; the stop face is a plain button. -->
<button
  type={face === "stop" ? "button" : "submit"}
  class="coin group relative block size-coin shrink-0"
  data-up={face === "stop" ? "stop" : "send"}
  aria-label={name}
  aria-disabled={face === "idle" ? "true" : undefined}
  title={face === "idle" ? say($lang, "talk_enter_hint") : undefined}
  onclick={(event) => {
    if (face === "idle") {
      event.preventDefault();
      return;
    }
    if (face === "stop") onStop();
  }}
>
  <span class="coin-faces" aria-hidden="true">
    <span
      class="coin-face bg-page text-text group-hover:wash-strong group-aria-disabled:text-text-disabled group-aria-disabled:group-hover:bg-page"
      data-face="send"
    >
      <Glyph name="send" size="key" />
    </span>
    <span class="coin-face bg-page text-text group-hover:wash-strong" data-face="stop">
      <Glyph name="stop" size="sm" solid />
    </span>
  </span>
</button>
