<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->
<script lang="ts" module>

  export interface Formula {
    readonly tex: string;
    readonly placed: "inline" | "display";
  }

  const DISPLAY = "$$";
  const INLINE = "$";

  export function formulaOf(source: string): Formula | null {
    const fence =
      source.startsWith(DISPLAY) && source.endsWith(DISPLAY) && source.length > 2 * DISPLAY.length
        ? DISPLAY
        : INLINE;
    if (!source.startsWith(fence) || !source.endsWith(fence) || source.length <= 2 * fence.length)
      return null;
    const tex = source.slice(fence.length, -fence.length);
    return tex.trim() === "" ? null : { tex, placed: fence === DISPLAY ? "display" : "inline" };
  }
</script>

<script lang="ts">

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

<span
  bind:this={host}
  class={drawn ? (formula?.placed === "display" ? "block overflow-x-auto" : "") : "hidden"}
></span>{#if !drawn}<code class="font-mono text-note text-text-faint">{source}</code>{/if}
