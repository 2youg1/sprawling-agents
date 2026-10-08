<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // The question `/quit` asks before the city closes (client/Spec.lean
  // §4-57c). With no run going it is close or cancel; with runs going it
  // is wait for them, stop them now, or cancel, the same three answers
  // the console's `/quit` gives. A page opened through the remote door
  // is told only that closing belongs to this machine, because the
  // relay refuses the verb from a device (`LocalOnly`).
  //
  // The platform owns the modality, as for `parts/dialog`: the first
  // control is the way out, so the safe answer is under the hand, and
  // `close()` gives the focus back to whatever held it.
  import { createAttachmentKey } from "svelte/attachments";
  import type { Attachment } from "svelte/attachments";

  import { fill, say } from "../core/lang";
  import { quitAsked } from "../core/quitting";
  import type { CloseMode } from "../wire";
  import { ui } from "../ui";
  import Button from "./parts/button.svelte";
  import type { Cancelling } from "./parts/sheet";
  import Sheet from "./parts/sheet.look.svelte";
  import { quitOf } from "./quit";

  const u = ui();
  const { lang } = u;
  const belief = u.conn.belief;
  const uid = $props.id();
  const HOLD = createAttachmentKey();

  let open = $state(false);
  // The count the page was opened with is no ask: only a press after it is.
  let seen = $quitAsked;
  $effect(() => {
    const asks = $quitAsked;
    if (asks === seen) return;
    seen = asks;
    open = true;
  });

  let sheet = $state<HTMLDialogElement | undefined>(undefined);
  const hold: Attachment<HTMLDialogElement> = (node) => {
    sheet = node;
    return () => {
      sheet = undefined;
    };
  };
  $effect(() => {
    if (sheet === undefined) return;
    if (open && !sheet.open) sheet.showModal();
    if (!open && sheet.open) sheet.close();
  });

  const look = $derived(quitOf({ runs: $belief.live.length, here: u.origin }));

  function close(mode: CloseMode): void {
    open = false;
    u.conn.closeCity(mode);
  }
</script>

<Sheet
  wire={{
    [HOLD]: hold,
    "aria-labelledby": `${uid}-title`,
    "aria-describedby": `${uid}-about`,
    oncancel: (event: Cancelling) => {
      event.preventDefault();
      open = false;
    },
  }}
  stands="centre"
>
  <h2 id={`${uid}-title`} class="text-heading text-text">{say($lang, "quit_title")}</h2>
  <p id={`${uid}-about`} class="text-note text-text-quiet">{say($lang, look.about)}</p>
  {#if look.runs > 0}
    <p class="text-note text-text">{fill(say($lang, "quit_runs"), { n: String(look.runs) })}</p>
  {/if}
  <div class="flex flex-wrap items-center justify-end gap-snug">
    <Button label={say($lang, "quit_cancel")} tone="secondary" onPress={() => {
        open = false;
      }} />
    {#each look.answers as answer (answer.mode)}
      <Button label={say($lang, answer.label)} tone={answer.tone} onPress={() => {
          close(answer.mode);
        }} />
    {/each}
  </div>
</Sheet>
