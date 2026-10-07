<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<!-- A fact on the composer's settings row, and the one shape every fact
there takes: no frame and no fill at rest, a wash under the pointer, so
the row reads as words under a line rather than as a toolbar
(docs/frontend-method.md §7I). Given a trigger's wire bag it is the
button that opens the fact's menu - the pill, the model entry, the
permissions entry; given none it is a read-only fact - the gate and the
sandbox. A fact a person must look at (a gate the city could not
report) carries the `asks` mark, which survives a forced colour mode
where a colour alone would not (client/Spec.lean §7C). -->
<script lang="ts">
  import type { Snippet } from "svelte";

  import Glyph from "../parts/glyph.svelte";
  import type { GlyphName } from "../parts/glyph";
  import type { TriggerWire } from "./pill";

  interface Props {
    readonly wire?: TriggerWire | undefined;
    readonly asks?: boolean;
    readonly glyph?: GlyphName | undefined;
    // Said to a screen reader before the words, when the drawn glyph is
    // what names the fact.
    readonly heard?: string | undefined;
    readonly children: Snippet;
  }

  const { wire, asks = false, glyph, heard, children }: Props = $props();

  const FACT = "fact inline-flex h-control-sm min-w-0 items-center gap-tight rounded-control px-snug text-note text-text-quiet";
</script>

{#if wire === undefined}
  <span class={[FACT, asks && "asks"]}>
    {#if glyph !== undefined}<Glyph name={glyph} size="sm" />{/if}
    {#if heard !== undefined}<span class="sr-only">{heard}</span>{/if}
    {@render children()}
  </span>
{:else}
  <button type="button" class="{FACT} trigger hover:wash hover:text-text aria-expanded:wash aria-expanded:text-text" {...wire}>
    {#if glyph !== undefined}<Glyph name={glyph} size="sm" />{/if}
    {@render children()}
  </button>
{/if}

<style>
  /* A fact never takes more than a short phrase's width, so one long
   * value cannot push the others off the row; its words truncate. */
  .fact {
    max-width: 16rem;
  }

  /* The wash and the brighter ink fade out when the pointer leaves and
   * arrive when it comes: the resting state leaves, the hovered and the
   * open states arrive (docs/frontend-method.md §4-43). */
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
