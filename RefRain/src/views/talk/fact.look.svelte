<script lang="ts">
  import type { Snippet } from "svelte";

  import Glyph from "../parts/glyph.svelte";
  import type { GlyphName } from "../parts/glyph";
  import type { TriggerWire } from "./fact";

  interface Props {
    readonly wire?: TriggerWire | undefined;
    readonly asks?: boolean;
    readonly glyph?: GlyphName | undefined;

    readonly heard?: string | undefined;
    readonly children: Snippet;
  }

  const { wire, asks = false, glyph, heard, children }: Props = $props();

  const FACT =
    "fact inline-flex h-control-sm min-w-0 items-center gap-tight rounded-control px-snug text-note text-text-quiet";
</script>

{#if wire === undefined}
  <span class={[FACT, asks && "asks"]}>
    {#if glyph !== undefined}<Glyph name={glyph} size="sm" />{/if}
    {#if heard !== undefined}<span class="sr-only">{heard}</span>{/if}
    {@render children()}
  </span>
{:else}
  <button
    type="button"
    class="{FACT} trigger hover:wash hover:text-text aria-expanded:wash aria-expanded:text-text"
    {...wire}
  >
    {#if glyph !== undefined}<Glyph name={glyph} size="sm" />{/if}
    {@render children()}
  </button>
{/if}

<style>
  .fact {
    max-width: 16rem;
  }

  .trigger {
    transition-property: background-color, color;
    transition-duration: var(--transition-duration-short);
    transition-timing-function: var(--ease-leave);
  }
  .trigger:hover,
  .trigger[aria-expanded="true"] {
    transition-timing-function: var(--ease-arrive);
  }
</style>
