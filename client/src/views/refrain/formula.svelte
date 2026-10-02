<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  // A formula the city read between dollar signs and left undrawn
  // (`crates/documents/Spec.lean` D22), drawn by KaTeX as MathML
  // (client/Spec.lean §4-64a). The source comes with its delimiters: two
  // dollar signs on each side is a display formula, one is inline.
  export interface Formula {
    readonly tex: string;
    readonly placed: "inline" | "display";
  }

  const DISPLAY = "$$";
  const INLINE = "$";

  export function formulaOf(source: string): Formula | null {
    const fence = source.startsWith(DISPLAY) && source.endsWith(DISPLAY) && source.length > 2 * DISPLAY.length ? DISPLAY : INLINE;
    if (!source.startsWith(fence) || !source.endsWith(fence) || source.length <= 2 * fence.length) return null;
    const tex = source.slice(fence.length, -fence.length);
    return tex.trim() === "" ? null : { tex, placed: fence === DISPLAY ? "display" : "inline" };
  }
</script>

<script lang="ts">
  // KaTeX arrives in a chunk of its own on the first formula a page
  // draws. It builds the MathML as elements into the one node this
  // component owns, never through `innerHTML` (4-26), and the browser's
  // own MathML engine lays it out, so no KaTeX stylesheet or font ships.
  // Until KaTeX answers, and wherever it refuses the source, the source
  // stays as it was written, in the mono face, as every construct this
  // page does not draw (appendix F: a command KaTeX does not know is
  // shown, not guessed at).
  import { Result } from "effect";

  interface Props {
    readonly source: string;
  }

  const { source }: Props = $props();

  const formula = $derived(formulaOf(source));
  let host = $state<HTMLElement | undefined>(undefined);
  let drawn = $state(false);

  $effect(() => {
    const into = host;
    const shown = formula;
    drawn = false;
    if (into === undefined || shown === null) return;
    let current = true;
    void import("katex").then(({ default: katex }) => {
      if (!current) return;
      drawn = Result.isSuccess(
        Result.try(() => {
          katex.render(shown.tex, into, {
            displayMode: shown.placed === "display",
            output: "mathml",
            throwOnError: true,
            strict: "ignore",
            trust: false,
          });
        }),
      );
    });
    return () => {
      current = false;
    };
  });
</script>

<!-- One line, so no space is drawn between a formula and the comma after it. -->
<span bind:this={host} class={drawn ? (formula?.placed === "display" ? "block overflow-x-auto" : "") : "hidden"}></span>{#if !drawn}<code class="font-mono text-note text-text-faint">{source}</code>{/if}
