<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  // How the coin key is drawn, and nothing else (docs/frontend-method.md
  // §7I): send and stop are two faces of one key that turns about its
  // vertical axis. With motion off or reduced the faces cross-fade in
  // place instead, so the answer to "what does this key do now" still
  // changes and nothing travels. The faint face says why it does nothing
  // in a hint beside it, which a keyboard reaches by focusing the key
  // and a screen reader reads with its name.
  import type { Snippet } from "svelte";

  import Glyph from "../parts/glyph.svelte";
  import Tip from "../parts/tip.svelte";
  import type { CoinLook } from "./coin";

  const look: CoinLook = $props();

  // The checker types a `{#snippet}` name as a void call, which the lint
  // lane rejects inside a render tag; the name is taken again as its
  // `Snippet` type, and the template renders that.
  const key: Snippet<[string | undefined]> = drawKey;
</script>

{#snippet drawKey(hint: string | undefined)}
  <button class="coin" data-up={look.face === "stop" ? "stop" : "send"} {...look.wire} aria-describedby={hint}>
    <span class="faces" aria-hidden="true">
      <span class="face" data-face="send"><Glyph name="send" size="key" /></span>
      <span class="face" data-face="stop"><Glyph name="stop" size="stop" solid /></span>
    </span>
  </button>
{/snippet}

<!-- The hint stands to the right: above the key is where the context
ring that surrounds it says its own reading. -->
{#if look.why === undefined}
  {@render key(undefined)}
{:else}
  <Tip text={look.why} side="right">
    {#snippet children(hint: string)}{@render key(hint)}{/snippet}
  </Tip>
{/if}

<style>
  .coin {
    position: relative;
    display: block;
    flex-shrink: 0;
    width: var(--spacing-coin);
    height: var(--spacing-coin);
    /* Seven and a half key widths away: near enough that the half turn
     * reads as a coin turning, far enough that its edge never fills the
     * key. */
    perspective: calc(var(--spacing-coin) * 7.5);
  }
  .faces {
    position: relative;
    display: block;
    width: 100%;
    height: 100%;
    transform-style: preserve-3d;
    transition: transform var(--transition-duration-coin) var(--ease-arrive);
  }
  .face {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
    border-radius: var(--radius-pill);
    background-color: var(--color-page);
    color: var(--color-text);
    backface-visibility: hidden;
    transition:
      background-color var(--transition-duration-short) var(--ease-leave),
      color var(--transition-duration-short) var(--ease-leave),
      opacity var(--transition-duration-short) var(--ease-leave);
  }
  .face[data-face="stop"] {
    transform: rotateY(180deg);
  }
  .coin[aria-disabled="true"] .face[data-face="send"] {
    color: var(--color-text-disabled);
  }
  /* Hover lays 8 % of the ink over the page colour itself, so the face
   * reads the same over any surface behind the composer; a key that
   * cannot be pressed keeps its ground. */
  .coin:hover .face {
    background-color: color-mix(in oklch, var(--color-text) 8%, var(--color-page));
    transition-timing-function: var(--ease-arrive);
  }
  .coin[aria-disabled="true"]:hover .face {
    background-color: var(--color-page);
  }
  .coin[data-up="stop"] .faces {
    transform: rotateY(180deg);
  }
  .coin:active .faces {
    scale: 0.94;
  }
  :global([data-motion="off"]) .faces,
  :global([data-motion="off"]) .face[data-face="stop"] {
    transform: none;
  }
  :global([data-motion="off"]) .face[data-face="stop"],
  :global([data-motion="off"]) .coin[data-up="stop"] .face[data-face="send"] {
    opacity: 0;
  }
  :global([data-motion="off"]) .coin[data-up="stop"] .face[data-face="stop"] {
    opacity: 1;
  }
  @media (prefers-reduced-motion: reduce) {
    :global(:root:not([data-motion="on"])) .faces,
    :global(:root:not([data-motion="on"])) .face[data-face="stop"] {
      transform: none;
    }
    :global(:root:not([data-motion="on"])) .face[data-face="stop"],
    :global(:root:not([data-motion="on"])) .coin[data-up="stop"] .face[data-face="send"] {
      opacity: 0;
    }
    :global(:root:not([data-motion="on"])) .coin[data-up="stop"] .face[data-face="stop"] {
      opacity: 1;
    }
  }
</style>
