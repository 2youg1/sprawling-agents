<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  import Look from "./typed_line.look.svelte";

  interface Props {
    readonly text: string;
    readonly box: HTMLTextAreaElement | undefined;
    readonly lit: boolean;
  }

  const { text, box, lit }: Props = $props();

  const measuring = () => document.createElement("canvas").getContext("2d");
  let ruler: ReturnType<typeof measuring> | undefined;

  function reach(words: string, field: HTMLTextAreaElement): number {
    ruler ??= measuring();
    const pen = ruler;
    if (pen === null) return field.clientWidth;
    pen.font = getComputedStyle(field).font;
    const widest = Math.max(...words.split("\n").map((line) => pen.measureText(line).width));
    return Math.min(widest + 2, field.clientWidth);
  }

  const typed = $derived(text === "" || box === undefined ? 0 : reach(text, box));
</script>

<Look {typed} {lit} />
