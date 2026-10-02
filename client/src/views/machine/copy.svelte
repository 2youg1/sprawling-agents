<script lang="ts" module>
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The copy control of a command, and its receipt. The face turns into a
// check for a moment after the write lands (ux-upgrades A7), so a press
// that quietly failed shows no check rather than a lying one. Two marks
// and no motion between them: the receipt is a cut, and the mark is
// `parts/glyph`'s `check`, which is where every icon in this client
// lives (client-SPEC 4-34).

// How long the receipt holds its check mark: long enough to see one,
// short enough that the mark never becomes the button's face.
const RECEIPT_MS = 1200;

// The resting paint, at the ordinary control height so the copy stands
// level with the Button beside it in a command row. The `::before`
// widens the touch surface to the 44-point floor while the drawn
// control stays on the control scale (client-SPEC 4-34).
const WEAR =
  "relative flex h-control shrink-0 items-center gap-tight rounded-control px-base " +
  "text-label text-text-quiet before:absolute before:-inset-snug before:content-[''] " +
  "transition-[background-color,color] hover:bg-raised " +
  "hover:text-text motion-reduce:transition-none";
</script>

<script lang="ts">
  import { say } from "../../core/lang";
  import { ui } from "../../ui";
  import Glyph from "../parts/glyph.svelte";

  interface Props {
    // The command, exactly as a person would type it.
    readonly text: string;
  }

  const { text }: Props = $props();

  const { lang } = ui();

  let copied = $state(false);
  let receipt: ReturnType<typeof setTimeout> | undefined = undefined;

  function copy(): void {
    void navigator.clipboard.writeText(text).then(() => {
      copied = true;
      if (receipt !== undefined) clearTimeout(receipt);
      receipt = setTimeout(() => {
        copied = false;
        receipt = undefined;
      }, RECEIPT_MS);
    });
  }
</script>

<button type="button" class={WEAR} aria-label={say($lang, copied ? "setup_copied" : "setup_copy")} onclick={copy}>
  {#if copied}
    <Glyph name="check" size="sm" class="shrink-0" />
  {:else}
    {say($lang, "setup_copy")}
  {/if}
</button>
