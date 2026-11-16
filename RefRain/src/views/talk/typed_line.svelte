<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  // The seat of the one-pixel line under the composer's words
  // (docs/frontend-method.md §7I): it says how far the words reach, and
  // `./typed_line.look.svelte` paints the line out to there.
  //
  // The width is measured rather than read off the box, because the box
  // is as wide as its column whatever is in it: a canvas with the box's
  // own font measures each line the way the box lays it out, and a line
  // wider than the box is the box's width.
  import Look from "./typed_line.look.svelte";

  interface Props {
    readonly text: string;
    readonly box: HTMLTextAreaElement | undefined;
    // Whether the box has the focus or holds words: the line is drawn at
    // full strength then, and half strength while nothing is happening.
    readonly lit: boolean;
  }

  const { text, box, lit }: Props = $props();

  // One measuring surface for the life of the component, made on first
  // use; its type is read off the call that makes it.
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
