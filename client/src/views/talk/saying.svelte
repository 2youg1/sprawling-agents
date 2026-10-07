<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // The words of a turn while they are still arriving. The city reads
  // the blocks that can no longer change (a list, a table, a code block)
  // as they close, so they are laid out as they close and the reply does
  // not jump when the record takes over (client/Spec.lean §4-26, §4-53); only
  // the open tail is drawn as raw text, its last few characters faint so
  // text emerges instead of appearing. No timer, no queue, no
  // per-character node.
  import Laid from "../refrain/laid.svelte";
  import { laidReply } from "../reply.svelte";
  import Look from "./saying.look.svelte";
  import NotePlace from "./note_place.svelte";
  import { faded } from "./saying";

  interface Props {
    readonly text: string;
    readonly who: string;
  }

  const { text, who }: Props = $props();

  const laid = laidReply(() => text, "streaming");
</script>

{#snippet blocks()}
  <Laid blocks={laid.blocks} />
{/snippet}

<NotePlace rhythm="shape">
  <Look {who} laid={blocks} {...faded(text.slice(laid.reached))} />
</NotePlace>
