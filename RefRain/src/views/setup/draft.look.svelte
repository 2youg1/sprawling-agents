<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  import type { DraftLook } from "./draft";

  const { wire }: DraftLook = $props();
</script>

<!-- The edge and fill of every text box on the page (`parts/field`), so
the document box reads as a field on the card rather than a hole in it;
it grows only downward, because the card sets its width. -->
<textarea
  {...wire}
  class="min-h-output w-full resize-y rounded-control border border-edge-input bg-raised px-base py-snug font-mono text-note text-text"
></textarea>
