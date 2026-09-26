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
  // reason: its second face.** After Enter it briefly reads "handed to
  // <room>" beside a check, so a person sees the send land (ux A3), and
  // the button part has no slot for a glyph. The paint is the primary
  // tier's, restated in tokens. An empty box leaves it drawn but inert,
  // so the place a person reaches for does not move.
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
    "flex h-control-lg items-center gap-snug rounded-control px-base text-label transition-[background-color,color,opacity,transform]",
    "duration-100 ease-standard active:scale-[0.98] motion-reduce:transition-none motion-reduce:active:scale-100",
    empty && !handed ? "bg-raised aria-disabled:text-text-disabled" : "bg-accent text-on-accent hover:bg-accent-hover",
  ]}
  aria-disabled={empty}
  onclick={(event) => {
    if (empty) event.preventDefault();
  }}
>
  <span role="status" class="flex items-center gap-tight">
    {#if handed}
      <Glyph name="check" size="sm" />
      {fill(say($lang, "talk_handed"), { room: here ?? "" })}
    {:else}
      {say($lang, SPELLING[sending])}
    {/if}
  </span>
</button>
