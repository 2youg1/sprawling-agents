<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  
  
  import Laid from "./editor/laid.svelte";
  import { laidReply } from "./reply.svelte";
  import { copiedSource } from "./talk/copying";

  interface Props {
    readonly text: string;
  }

  const { text }: Props = $props();

  const laid = laidReply(() => text, "settled");
  const rest = $derived(text.slice(laid.reached));

  
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
    <p class="my-snug whitespace-pre-wrap break-words font-read leading-read">{rest}</p>
  {/if}
</div>
