<!-- This Source Code Form is subject to the terms of the Mozilla Public
License, v. 2.0. If a copy of the MPL was not distributed with this
file, You can obtain one at https://mozilla.org/MPL/2.0/.
Copyright (c) 2026 2youg1 and the sprawling contributors -->

<!-- What a round reasoned, folded under one quiet line: the line is a row
of the thread, so it takes the row's wash under the pointer, and the
words open under a rule on their left. -->
<script lang="ts">
  import type { ReasoningLook } from "./reasoning";

  const look: ReasoningLook = $props();
</script>

<details class="text-note text-text-faint" open={look.open}>
  <summary class="cursor-pointer rounded-control px-tight marker:text-text-faint hover:wash hover:text-text-quiet">
    <span class="text-text-faint">{look.label}</span>
    {look.length}
  </summary>
  <div class="mt-tight border-l border-edge-panel pl-base whitespace-pre-wrap break-words">{look.text}</div>
</details>
