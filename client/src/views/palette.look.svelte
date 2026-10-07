<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  // How the palette's box is drawn, and nothing else (`./palette.ts`,
  // `BoxWire`): the line sunk one step into the raised sheet, as the
  // file finder's is, the list the seat hands over under it, and the
  // empty state when nothing matches. The sheet around all of it is
  // `modal.look.svelte`.
  import type { Snippet } from "svelte";

  import type { Key } from "../core/lang";
  import Empty from "./parts/empty.svelte";
  import type { BoxWire } from "./palette";

  interface Props {
    readonly box: BoxWire;
    readonly list: Snippet;
    // What the empty state says, present only while nothing matches.
    readonly nothing: Key | undefined;
  }

  const { box, list, nothing }: Props = $props();
</script>

<input {...box} class="w-full rounded-control bg-page px-base py-snug text-body placeholder:text-text-faint" />
{@render list()}
{#if nothing !== undefined}
  <Empty missing={nothing} />
{/if}
