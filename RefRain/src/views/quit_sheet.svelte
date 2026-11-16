<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  import type { CloseMode } from "../wire";
  import type { QuitLook } from "./quit";

  // The `/quit` question as drawn (client/Spec.lean §4-57c). Two seats,
  // as the key sheet has: `modal` is the one `quit.svelte` opens over the
  // page; `specimen` is the same box drawn open in a gallery fold's flow,
  // where it takes no focus and answers no Escape.
  export interface QuitSheetProps {
    readonly look: QuitLook;
    readonly seat: "modal" | "specimen";
    // Whether the modal stands; a specimen always does.
    readonly open: boolean;
    readonly onCancel: () => void;
    readonly onClose: (mode: CloseMode) => void;
  }
</script>

<script lang="ts">
  import { createAttachmentKey } from "svelte/attachments";
  import type { Attachment } from "svelte/attachments";

  import { fill, say } from "../core/lang";
  import { ui } from "../ui";
  import Button from "./parts/button.svelte";
  import type { Cancelling } from "./parts/sheet";
  import Sheet from "./parts/sheet.look.svelte";

  const { look, seat, open, onCancel, onClose }: QuitSheetProps = $props();
  const { lang } = ui();
  const uid = $props.id();
  const HOLD = createAttachmentKey();

  // The platform owns the modality: `showModal()` traps the focus and
  // marks the page inert, and `close()` gives the focus back to whatever
  // held it before the question opened.
  let sheet = $state<HTMLDialogElement | undefined>(undefined);
  const hold: Attachment<HTMLDialogElement> = (node) => {
    sheet = node;
    return () => {
      sheet = undefined;
    };
  };
  $effect(() => {
    if (sheet === undefined || seat === "specimen") return;
    if (open && !sheet.open) sheet.showModal();
    if (!open && sheet.open) sheet.close();
  });
</script>

<Sheet
  wire={{
    [HOLD]: hold,
    open: seat === "specimen",
    "aria-labelledby": `${uid}-title`,
    "aria-describedby": `${uid}-about`,
    oncancel: (event: Cancelling) => {
      event.preventDefault();
      onCancel();
    },
  }}
  stands={seat === "specimen" ? "in-flow" : "centre"}
>
  <h2 id={`${uid}-title`} class="text-heading text-text">{say($lang, "quit_title")}</h2>
  <p id={`${uid}-about`} class="text-note text-text-quiet">{say($lang, look.about)}</p>
  {#if look.runs > 0}
    <p class="text-note text-text">{fill(say($lang, "quit_runs"), { n: String(look.runs) })}</p>
  {/if}
  <div class="flex flex-wrap items-center justify-end gap-snug">
    <Button label={say($lang, "quit_cancel")} tone="secondary" onPress={onCancel} />
    {#each look.answers as answer (answer.mode)}
      <Button
        label={say($lang, answer.label)}
        tone={answer.tone}
        onPress={() => {
          onClose(answer.mode);
        }}
      />
    {/each}
  </div>
</Sheet>
