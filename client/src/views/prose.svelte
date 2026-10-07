<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // A settled reply, laid out by the city's one Markdown grammar
  // (`Query::Reply`, client/Spec.lean §4-26, §4-53) and drawn by the same
  // renderer as RefRain's preview. Until the city has read a stretch it
  // is drawn as it is, so a page with no city still shows every word.
  //
  // **Width is the caller's.** The same reading draws a model's reply
  // inside the talk thread (the `talk` tier) and a Markdown file inside
  // the building page's file view (the `measure` tier), and the tier
  // belongs to the screen that chose the container (docs/frontend-method.md §4-33).
  // A table or a code block never wraps per character - it scrolls
  // inside its own box, which `refrain/laid.svelte` holds.
  import Laid from "./refrain/laid.svelte";
  import { laidReply } from "./reply.svelte";
  import { copiedSource } from "./talk/copying";

  interface Props {
    readonly text: string;
  }

  const { text }: Props = $props();

  const laid = laidReply(() => text, "settled");
  const rest = $derived(text.slice(laid.reached));

  // Copying the whole of it hands over its Markdown (talk/copying.ts);
  // a selection that reaches past it is the page's, not this reply's.
  function copy(event: ClipboardEvent): void {
    const drawn = event.currentTarget;
    const chosen = document.getSelection();
    if (!(drawn instanceof HTMLElement) || chosen?.rangeCount !== 1) return;
    if (!drawn.contains(chosen.getRangeAt(0).commonAncestorContainer)) return;
    const source = copiedSource(chosen.toString(), drawn.textContent, text);
    if (source === null || event.clipboardData === null) return;
    event.clipboardData.setData("text/plain", source);
    event.preventDefault();
  }
</script>

<div oncopy={copy}>
  <Laid blocks={laid.blocks} />
  {#if rest !== ""}
    <p class="my-snug whitespace-pre-wrap break-words font-read leading-relaxed">{rest}</p>
  {/if}
</div>
