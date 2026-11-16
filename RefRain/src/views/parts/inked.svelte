<script lang="ts" module>
  // This Source Code Form is subject to the terms of the Mozilla Public
  // License, v. 2.0. If a copy of the MPL was not distributed with this
  // file, You can obtain one at https://mozilla.org/MPL/2.0/.
  // Copyright (c) 2026 2youg1 and the sprawling contributors
</script>

<script lang="ts">
  import { painted, type Piece } from "./code";
  import Look from "./inked.look.svelte";

  interface Props {
    readonly text: string;
    readonly source: string;
  }

  const { text, source }: Props = $props();

  interface Answer {
    readonly text: string;
    readonly source: string;
    readonly pieces: readonly Piece[];
  }

  let answer = $state<Answer | null>(null);

  const pieces: readonly Piece[] = $derived(
    answer !== null && answer.text === text && answer.source === source
      ? answer.pieces
      : [{ ink: "plain", text }],
  );

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
