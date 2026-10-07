<!-- This Source Code Form is subject to the terms of the Mozilla Public
License, v. 2.0. If a copy of the MPL was not distributed with this
file, You can obtain one at https://mozilla.org/MPL/2.0/.
Copyright (c) 2026 2youg1 and the sprawling contributors -->

<!-- How the design questions waiting for the person are drawn
(`WaitingLook`, `./waiting`): each on the one decide card the things
that stop and ask share (`parts/decide.svelte`, client/Spec.lean §7C),
its body the question, what it is about, what denying keeps and the
link to the bin, then how many questions the answer covers and the
tainted mark. The cards are a pane apart. -->
<script lang="ts">
  import Badge from "../parts/badge.svelte";
  import Decide from "../parts/decide.svelte";
  import type { WaitingLook } from "./waiting";

  const { cards, asked }: WaitingLook = $props();
</script>

<div class="flex flex-col gap-pane">
  {#each cards as card (card.key)}
    <Decide
      kind="question"
      asker={card.asker}
      at={card.at}
      choices={[
        { answer: "yes", ...card.allow },
        { answer: "no", ...card.deny },
      ]}
    >
      {#snippet body()}
        <p class="text-body leading-relaxed">{card.action}</p>
        {@render asked(card.locator)}
        <p class="mt-snug text-note text-text-faint">
          {card.keeps}
          <a class="bin underline" href={card.bin.href}>{card.bin.text}</a>
        </p>
        {#if card.same !== undefined || card.tainted !== undefined}
          <p class="mt-snug flex flex-wrap items-center gap-snug text-note">
            {#if card.same !== undefined}
              <span class="text-text-faint">{card.same}</span>
            {/if}
            {#if card.tainted !== undefined}
              <Badge text={card.tainted} weight="quiet" />
            {/if}
          </p>
        {/if}
      {/snippet}
    </Decide>
  {/each}
</div>

<style>
  .bin {
    transition: color var(--transition-duration-short) var(--ease-leave);
  }
  .bin:hover {
    color: var(--color-text);
    transition-timing-function: var(--ease-arrive);
  }
</style>
