<script lang="ts" module>
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The copy control of a command, and its receipt. Every press-to-copy
// key in the client is this one control, so it carries the three marks
// D55 asks of a button once: a recognisable icon (`parts/glyph`'s
// `copy`, where every icon in this client lives,
// docs/frontend-method.md §4-34), a visible name, and a note that says
// what lands on the clipboard (`parts/tip`, shown on hover and on
// keyboard focus, because a touch screen has no hover). The note
// quotes the text, so a row with two copy keys says which is which
// without the User pressing either.
//
// The face turns into a check and the name into "copied" for a moment
// after the write lands (ux-upgrades A7), so a press that quietly
// failed shows no check rather than a lying one. Two faces and no
// motion between them: the receipt is a cut.

// How long the receipt holds its check mark: long enough to see one,
// short enough that the mark never becomes the button's face.
const RECEIPT_MS = 1200;

// The resting paint, at the ordinary control height so the copy stands
// level with the Button beside it in a command row. The `::before`
// widens the touch surface to the 44-point floor while the drawn
// control stays on the control scale (docs/frontend-method.md §4-34).
const WEAR =
  "relative flex h-control shrink-0 items-center gap-tight rounded-control px-snug " +
  "text-label text-text-quiet before:absolute before:-inset-snug before:content-[''] " +
  "transition-[background-color,color] hover:bg-raised " +
  "hover:text-text still:transition-none";
</script>

<script lang="ts">
  import { fill, say } from "../../core/lang";
  import { ui } from "../../ui";
  import Glyph from "../parts/glyph.svelte";
  import Tip from "../parts/tip.svelte";

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

<Tip text={fill(say($lang, "copy_note"), { text })}>
  {#snippet children(hint: string)}
    <button type="button" class={WEAR} aria-describedby={hint} onclick={copy}>
      <Glyph name={copied ? "check" : "copy"} size="sm" class="shrink-0" />
      {say($lang, copied ? "setup_copied" : "setup_copy")}
    </button>
  {/snippet}
</Tip>
