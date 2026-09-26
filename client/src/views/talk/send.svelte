<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->
<script lang="ts">
  // The control that sends what the composer holds.
  //
  // **It is drawn here rather than in `parts/button.svelte` for one
  // reason: its second face.** After Enter the arrow becomes a check and
  // a status line says "handed to <room>", so a person sees the send land
  // (ux A3), and the button part has no slot for a glyph. It is a round
  // arrow at the box's right edge, where every chat page puts it, and its
  // name is the verb it sends. An empty box leaves it drawn but inert, so
  // the place a person reaches for does not move.
  import type { Sending } from "../../core/doing";
  import { fill, say } from "../../core/lang";
  import { ui } from "../../ui";
  import type { Address } from "../../wire";
  import Glyph from "../parts/glyph.svelte";
  import { SPELLING } from "./composer";

  interface SendProps {
    readonly sending: Sending;
    readonly handed: boolean;
    readonly here: Address | null;
    readonly empty: boolean;
  }

  const { sending, handed, here, empty }: SendProps = $props();
  const { lang } = ui();
</script>

<button
  type="submit"
  class={[
    "relative flex size-control shrink-0 items-center justify-center rounded-pill transition-[background-color,color,opacity,transform] before:absolute before:-inset-tight before:content-['']",
    "duration-100 ease-standard active:scale-[0.96] motion-reduce:transition-none motion-reduce:active:scale-100",
    empty && !handed ? "bg-raised-hover aria-disabled:text-text-disabled" : "bg-accent text-on-accent hover:bg-accent-hover",
  ]}
  aria-label={say($lang, SPELLING[sending])}
  aria-disabled={empty}
  onclick={(event) => {
    if (empty) event.preventDefault();
  }}
>
  <Glyph name={handed ? "check" : "send"} />
</button>
<span role="status" class="sr-only">
  {#if handed}{fill(say($lang, "talk_handed"), { room: here ?? "" })}{/if}
</span>
