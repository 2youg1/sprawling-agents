<!-- This Source Code Form is subject to the terms of the Mozilla Public
License, v. 2.0. If a copy of the MPL was not distributed with this
file, You can obtain one at https://mozilla.org/MPL/2.0/.
Copyright (c) 2026 2youg1 and the sprawling contributors -->

<!-- How the head of one message is drawn (`HeadLook`, `./head`): one
line of note text, the speaker in the label weight and everything else
faint, so a thread reads as a column of names with words under them.

Inline flow rather than one clipped row: the first head carries the
session's frozen facts, which can outrun the column, and a fact cut off
by an ellipsis is a fact the head no longer states. A space and the
tight margin make the gap the row had; short figures keep their words
together, and the facts wrap where the column ends. -->
<script lang="ts">
  import type { HeadLook } from "./head";

  const { who, model, time, ttft, tps, rhythm }: HeadLook = $props();
</script>

<div class="min-w-0 text-note text-text-faint">
  <span class="me-tight font-label text-label whitespace-nowrap text-text">{who}</span>
  {#if model !== null}
    <span class="me-tight wrap-anywhere">{model}</span>
  {/if}
  <time class="figure me-tight whitespace-nowrap" datetime={time.iso}>{time.text}</time>
  {#if ttft !== undefined}
    <span class="figure me-tight whitespace-nowrap">{ttft}</span>
  {/if}
  {#if tps !== undefined}
    <span class="figure me-tight whitespace-nowrap">{tps}</span>
  {/if}
  {@render rhythm?.()}
</div>
