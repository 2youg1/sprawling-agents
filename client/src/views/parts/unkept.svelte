<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  // A draft the browser would not store (refrain §3-14, the second row;
  // client/Spec.lean §4-63): one line that says the words live in this tab
  // alone and are gone when it closes, and a copy beside it, so the
  // person can carry them out before that. The box and RefRain draw it
  // under what they hold.

  // How long the copy receipt holds: long enough to see, short enough
  // that it never becomes the control's face.
  const RECEIPT_MS = 1200;
</script>

<script lang="ts">
  import { say } from "../../core/lang";
  import { ui } from "../../ui";

  interface Props {
    // The words as they stand when the copy is pressed.
    readonly words: () => string;
  }

  const { words }: Props = $props();

  const lang = ui().lang;

  let copied = $state(false);

  // The receipt appears only after the write landed, so a press that
  // quietly failed shows no receipt rather than a lying one.
  function copy(): void {
    void navigator.clipboard.writeText(words()).then(() => {
      copied = true;
      setTimeout(() => {
        copied = false;
      }, RECEIPT_MS);
    });
  }
</script>

<p class="flex items-baseline gap-base text-note text-alert" role="status">
  <span class="min-w-0">{say($lang, "part_unkept")}</span>
  <button type="button" class="shrink-0 rounded-control px-snug text-text-quiet hover:bg-raised hover:text-text" onclick={copy}>
    {say($lang, copied ? "code_copied" : "code_copy")}
  </button>
</p>
