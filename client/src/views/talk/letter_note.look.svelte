<!-- This Source Code Form is subject to the terms of the Mozilla Public
License, v. 2.0. If a copy of the MPL was not distributed with this
file, You can obtain one at https://mozilla.org/MPL/2.0/.
Copyright (c) 2026 2youg1 and the sprawling contributors -->

<!-- How a resident's letter is drawn (`LetterLook`, `./note`), as a
letter rather than as the person's bubble (client D86): left-aligned,
unfilled, held to the same five sixths of the column, with an accent
edge on the side it came from, its head a note's row over the words. -->
<script lang="ts">
  import NoteRow from "./note.look.svelte";
  import type { LetterLook } from "./note";

  const { label, head, said }: LetterLook = $props();
</script>

<article
  class="mr-auto max-w-5/6 rounded-card border border-edge-panel border-l-2 border-l-accent px-pane py-base"
  aria-label={label}
>
  <div class="mb-tight">
    <NoteRow role={undefined} pieces={head} />
  </div>
  <div class="text-body leading-relaxed whitespace-pre-wrap wrap-anywhere">{said}</div>
</article>
