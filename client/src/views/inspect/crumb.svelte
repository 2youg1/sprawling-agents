<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  // The line above a diff in the editor region (docs/frontend-method.md §7F): the
  // file's folder and name, the two trees being compared, and at the
  // right end the way to the person's own editor.
</script>

<script lang="ts">
  import type { Snippet } from "svelte";

  import type { Shown } from "../../core/editor";
  import Reach from "./reach.svelte";

  interface Props {
    // Relative to the city, as the ledger records it.
    readonly path: string;
    // The line the reach control names, one-based.
    readonly line: number;
    readonly shown: Shown;
    readonly children?: Snippet;
  }

  const { path, line, shown, children }: Props = $props();

  const cut = $derived(path.lastIndexOf("/") + 1);
</script>

<div
  class="flex h-[calc(4*var(--spacing-baseline))] shrink-0 items-center gap-base border-b border-edge pr-snug pl-wide text-note text-text-faint"
>
  <span class="min-w-0 flex-1 truncate font-mono"
    >{path.slice(0, cut)}<span class="font-label text-text-quiet">{path.slice(cut)}</span></span
  >
  {#if children !== undefined}<span class="shrink-0 whitespace-nowrap">{@render children()}</span>{/if}
  <Reach {path} {line} {shown} />
</div>
