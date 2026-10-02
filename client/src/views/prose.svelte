<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // A settled reply, laid out by the city's one Markdown grammar
  // (`Query::Reply`, client-SPEC 4-26, 4-53) and drawn by the same
  // renderer as RefRain's preview. Until the city has read a stretch it
  // is drawn as it is, so a page with no city still shows every word.
  //
  // **Width is the caller's.** The same reading draws a model's reply
  // inside the talk thread (the `talk` tier) and a Markdown file inside
  // the building page's file view (the `measure` tier), and the tier
  // belongs to the screen that chose the container (client-SPEC 4-33).
  // A table or a code block never wraps per character - it scrolls
  // inside its own box, which `refrain/laid.svelte` holds.
  import Laid from "./refrain/laid.svelte";
  import { laidReply } from "./reply.svelte";

  interface Props {
    readonly text: string;
  }

  const { text }: Props = $props();

  const laid = laidReply(() => text, "settled");
  const rest = $derived(text.slice(laid.reached));
</script>

<div>
  <Laid blocks={laid.blocks} />
  {#if rest !== ""}
    <p class="my-snug whitespace-pre-wrap break-words leading-relaxed">{rest}</p>
  {/if}
</div>
