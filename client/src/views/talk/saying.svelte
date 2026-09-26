<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // The words of a turn while they are still arriving. Blocks that can no
  // longer change (a list, a table, a code block) are laid out as they
  // close, so the reply does not jump when the record takes over; only
  // the open tail is drawn as raw text, its last few characters faint so
  // text emerges instead of appearing. Derived from the text and nothing
  // else: no timer, no queue, no per-character node.
  import { closedUpTo } from "../../core/prose";
  import Prose from "../prose.svelte";

  interface Props {
    readonly text: string;
    readonly who: string;
  }

  const { text, who }: Props = $props();

  // How many characters at the growing edge are drawn faint. Wide enough
  // that text emerges instead of appearing, narrow enough that the band a
  // reader's eye sits on is not the shimmering one.
  const EDGE = 10;

  // `laid` is a string, so `Prose` re-reads it only when a block closes,
  // not on every token.
  const laid = $derived(text.slice(0, closedUpTo(text)));
  const open = $derived(text.slice(laid.length));
  const settled = $derived(open.slice(0, -EDGE));
  const edge = $derived(open.slice(-EDGE));
</script>

<div class="my-base text-body">
  <div class="mb-tight text-note text-text-disabled">{who}</div>
  {#if laid !== ""}
    <Prose text={laid} />
  {/if}
  <div class="whitespace-pre-wrap leading-relaxed">
    {settled}<span class="text-text-faint">{edge}</span><span
      class="blink ml-tight inline-block size-[6px] bg-accent align-baseline"
    ></span>
  </div>
</div>
