<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // The card that stops and asks, drawn: a 2 px bar down the leading
  // edge (`asks`, theme/surface.css) and a glyph, a heading that says
  // who asks and when, the kind's own body, and at most three answers,
  // each with its chord beside it. A person learns the card once; every
  // later one is recognised rather than read.
  import Button from "./button.svelte";
  import Glyph from "./glyph.svelte";
  import Kbd from "./kbd.look.svelte";
  import type { DecideLook } from "./decide";

  const { card, glyph, asker, at, body, answers }: DecideLook = $props();
</script>

<div class="asks flex min-w-0 flex-col gap-snug rounded-card py-base pr-pane focus-visible:wash" {...card}>
  <div class="flex min-w-0 items-center gap-snug text-note">
    <Glyph name={glyph} size="sm" class="shrink-0 text-alert" />
    <span id={asker.id} class="min-w-0 flex-1 truncate text-text-quiet">{asker.text}</span>
    <span class="figure shrink-0 text-text-faint">{at}</span>
    <!-- The digit a list of entries draws beside each one; empty
    wherever the card is not in such a list (theme/mailbox.css). -->
    <kbd class="entry-n" aria-hidden="true"></kbd>
  </div>
  <div class="min-w-0">{@render body()}</div>
  {#if answers.length > 0}
    <div class="flex flex-wrap items-center justify-end gap-snug">
      {#each answers as answer (answer.answer)}
        <span class="inline-flex items-center gap-tight">
          <Button
            label={answer.label}
            tone={answer.tone}
            {...answer.why === undefined ? {} : { why: answer.why }}
            onPress={answer.onPress}
          />
          <span aria-hidden="true"><Kbd marks={answer.marks} /></span>
        </span>
      {/each}
    </div>
  {/if}
</div>
