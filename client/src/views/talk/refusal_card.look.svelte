<!-- This Source Code Form is subject to the terms of the Mozilla Public
License, v. 2.0. If a copy of the MPL was not distributed with this
file, You can obtain one at https://mozilla.org/MPL/2.0/.
Copyright (c) 2026 2youg1 and the sprawling contributors -->

<!-- How a refusal is drawn in the thread (`CardLook`, `./refused`): a
card with a faint alert edge, its first line led by the alert ink, the
lines under it, the city's own sentence one fold away in the mono face,
and the ways out in one row - a place to go as an underlined link, an
action as a button. -->
<script lang="ts">
  import Button from "../parts/button.svelte";
  import type { CardLook } from "./refused";

  const { role, head, after, lines, fold, ways }: CardLook = $props();
</script>

<div class="rounded-card border border-alert/40 px-base py-snug text-note text-text-quiet" {role}>
  <p><span class="text-alert">{head}</span>{after === undefined ? "" : ` · ${after}`}</p>
  {#each lines as line, at (at)}
    <p class={["mt-tight", line.ink === "faint" ? "text-text-faint" : "text-text-quiet"]}>{line.text}</p>
  {/each}
  {#if fold !== undefined}
    <details class="mt-tight min-w-0">
      <summary class="fold cursor-pointer text-text-faint">{fold.summary}</summary>
      <p class="mt-tight font-mono wrap-anywhere">{fold.text}</p>
    </details>
  {/if}
  {#if ways.length > 0}
    <div class="mt-snug flex flex-wrap items-center gap-base">
      {#each ways as way, at (at)}
        {#if way.kind === "link"}
          <a href={way.href} class="way underline">{way.text}</a>
        {:else}
          <Button label={way.label} onPress={way.onPress} />
        {/if}
      {/each}
    </div>
  {/if}
</div>

<style>
  .way,
  .fold {
    transition: color var(--transition-duration-short) var(--ease-leave);
  }
  .way:hover,
  .fold:hover {
    color: var(--color-text);
    transition-timing-function: var(--ease-arrive);
  }
</style>
