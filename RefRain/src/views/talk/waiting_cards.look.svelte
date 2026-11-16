

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
        <p class="text-body leading-read">{card.action}</p>
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
