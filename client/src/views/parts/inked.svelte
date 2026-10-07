<script lang="ts" module>
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// Code drawn in its inks, meant to sit inside a `<pre>` the caller
// owns. The text is on screen at once in plain ink and takes its
// colours when the highlighter answers, so a slow chunk delays colour
// and never the words. This is the seat: it asks the highlighter and
// draws whatever `./inked.look.svelte` is.
</script>

<script lang="ts">
  import { painted, type Piece } from "./code";
  import Look from "./inked.look.svelte";

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

<Look {pieces} />
