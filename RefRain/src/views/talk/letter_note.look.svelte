<!-- This Source Code Form is subject to the terms of the Mozilla Public
License, v. 2.0. If a copy of the MPL was not distributed with this
file, You can obtain one at https://mozilla.org/MPL/2.0/.
Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  import NoteRow from "./note.look.svelte";
  import type { LetterLook } from "./note";
  const { label, head, said }: LetterLook = $props();
</script>

<article
  class="mr-auto max-w-5/6 rounded-card border border-edge-panel px-pane py-base"
  aria-label={label}
>
  <div class="mb-tight">
    <NoteRow role={undefined} pieces={head} />
  </div>
  <div class="text-body leading-read whitespace-pre-wrap wrap-anywhere">{said}</div>
</article>
