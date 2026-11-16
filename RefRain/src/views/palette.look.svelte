<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  import type { Snippet } from "svelte";

  import type { Key } from "../core/lang";
  import Empty from "./parts/empty.svelte";
  import type { BoxWire } from "./palette";

  interface Props {
    readonly box: BoxWire;
    readonly list: Snippet;
    readonly nothing: Key | undefined;
    readonly foot?: Snippet | undefined;
  }

  const { box, list, nothing, foot }: Props = $props();
</script>

<input
  {...box}
  class="w-full rounded-control bg-page px-base py-snug text-body placeholder:text-text-faint"
/>
{@render list()}
{#if nothing !== undefined}
  <Empty missing={nothing} />
{/if}
{#if foot !== undefined}
  <div class="mt-snug border-t border-edge-panel pt-snug">{@render foot()}</div>
{/if}
