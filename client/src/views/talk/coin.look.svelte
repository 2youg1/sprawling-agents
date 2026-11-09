<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  // How the coin key is drawn, and nothing else (docs/frontend-method.md
  // §7I): send and pause are two faces of one key that turns about its
  // vertical axis. With motion off or reduced the faces cross-fade in
  // place instead. The idle face is not drawn — an empty box with no
  // run leaves the ring alone. The tip always names the command a press
  // would send, so the key and the context ring stay distinguishable.
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
  <button
    class="coin"
    data-up={look.face === "stop" ? "stop" : "send"}
    data-kind={look.face === "send" ? look.sending : undefined}
    {...look.wire}
    aria-describedby={hint}
  >
    <span class="faces" aria-hidden="true">
      <span class="face" data-face="send"><Glyph name="send" size="key" /></span>
      <span class="face" data-face="stop"><Glyph name="stop" size="stop" solid /></span>
    </span>
  </button>
{/snippet}

{#if look.face !== "idle"}
  {#if look.command === undefined}
    {@render key(undefined)}
  {:else}
    <Tip text={look.command} side="right">
      {#snippet children(hint: string)}{@render key(hint)}{/snippet}
    </Tip>
  {/if}
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
    transition:
      scale var(--transition-duration-short) var(--ease-leave),
      color var(--transition-duration-short) var(--ease-leave);
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
      opacity var(--transition-duration-short) var(--ease-leave),
      scale var(--transition-duration-short) var(--ease-leave);
  }
  .face[data-face="stop"] {
    transform: rotateY(180deg);
  }
  /* Hover lifts the wash and grows the key a hair; a press yields by the
   * theme's press scale; release returns on the leave curve. */
  .coin:hover .face {
    background-color: color-mix(in oklch, var(--color-text) 8%, var(--color-page));
    transition-timing-function: var(--ease-arrive);
  }
  .coin:hover {
    scale: 1.06;
    transition-timing-function: var(--ease-arrive);
  }
  .coin[data-up="stop"] .faces {
    transform: rotateY(180deg);
  }
  /* Dispatch takes the accent; steer keeps the quieter ink so the two
   * send kinds stay distinguishable before the tip is read. */
  .coin[data-kind="dispatch"] .face[data-face="send"] {
    color: var(--color-accent);
  }
  .coin:active {
    scale: var(--scale-press);
    transition-timing-function: var(--ease-arrive);
  }
  .coin:active .faces {
    scale: 1;
  }
  :global([data-motion="off"]) .coin,
  :global([data-motion="off"]) .faces,
  :global([data-motion="off"]) .face {
    transition: none;
  }
  :global([data-motion="off"]) .coin:hover,
  :global([data-motion="off"]) .coin:active {
    scale: 1;
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
    :global(:root:not([data-motion="on"])) .coin,
    :global(:root:not([data-motion="on"])) .faces,
    :global(:root:not([data-motion="on"])) .face {
      transition: none;
    }
    :global(:root:not([data-motion="on"])) .coin:hover,
    :global(:root:not([data-motion="on"])) .coin:active {
      scale: 1;
    }
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
