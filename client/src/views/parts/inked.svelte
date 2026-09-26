<script lang="ts" module>
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// Code drawn in its inks, meant to sit inside a `<pre>` the caller
// owns. The text is on screen at once in plain ink and takes its
// colours when the highlighter answers, so a slow chunk delays colour
// and never the words. The file view and a Markdown code block both
// draw through here, which is why the ink table lives here.

import type { Ink } from "./code";

// Every colour comes from `theme.css`; this file states no value. The
// fifth ink is the plain text around the four the theme distinguishes.
const PAINT: Record<Ink, string> = {
  plain: "",
  comment: "text-text-disabled",
  string: "text-alert",
  number: "text-accent",
  word: "text-text",
};
</script>

<script lang="ts">
  import { painted, type Piece } from "./code";

  interface Props {
    readonly text: string;
    // A file path or a Markdown fence word; it picks the grammar.
    readonly source: string;
  }

  const { text, source }: Props = $props();

  interface Answer {
    readonly text: string;
    readonly source: string;
    readonly pieces: readonly Piece[];
  }

  let answer = $state<Answer | null>(null);

  // Plain ink until the answer for this very text and source is in, so
  // the words are drawn on the first frame and never blank.
  const pieces: readonly Piece[] = $derived(
    answer !== null && answer.text === text && answer.source === source ? answer.pieces : [{ ink: "plain", text }],
  );

  // An answer that arrives after the text moved on is dropped, so a
  // slow grammar can never paint the previous file over the current one.
  $effect(() => {
    const asked = { text, source };
    let current = true;
    void painted(asked.text, asked.source).then((said) => {
      if (current) answer = { ...asked, pieces: said };
    });
    return () => {
      current = false;
    };
  });
</script>

{#each pieces as piece (piece)}<span class={PAINT[piece.ink]}>{piece.text}</span>{/each}
