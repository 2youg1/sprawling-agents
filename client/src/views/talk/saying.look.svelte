<!-- This Source Code Form is subject to the terms of the Mozilla Public
License, v. 2.0. If a copy of the MPL was not distributed with this
file, You can obtain one at https://mozilla.org/MPL/2.0/.
Copyright (c) 2026 2youg1 and the sprawling contributors -->

<!-- How the words of a turn still arriving are drawn (`SayingLook`,
`./saying`): the speaker in the note's faint ink, the laid-out blocks,
then the open tail as raw text, its last characters faint, and after
it the caret - an accent square, `--spacing-caret` across, sitting on
the line's baseline. The caret blinks with the theme's `blink`, which
the person's motion choice and the machine's reduced motion both
still, so a stilled caret stands lit rather than vanishing. -->
<script lang="ts">
  import type { SayingLook } from "./saying";

  const { who, laid, settled, edge }: SayingLook = $props();
</script>

<div class="text-body">
  <div class="mb-tight text-note text-text-faint">{who}</div>
  {@render laid()}
  <div class="whitespace-pre-wrap break-words leading-relaxed">
    {settled}<span class="text-text-faint">{edge}</span><span class="caret blink bg-accent"></span>
  </div>
</div>

<style>
  .caret {
    display: inline-block;
    width: var(--spacing-caret);
    height: var(--spacing-caret);
    margin-inline-start: var(--spacing-tight);
    vertical-align: baseline;
  }
</style>
