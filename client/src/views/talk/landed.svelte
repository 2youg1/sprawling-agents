<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  // One line where the person sent from, when the work they dispatched
  // started in another room: the room is a link, and the page stays put
  // so the box and its draft stay under the person's hands
  // (client D6). A run that started here needs no line, because
  // the thread already draws it.
  import { say } from "../../core/lang";
  import type { Landing } from "../../core/landing";
  import { toFragment } from "../../core/route";
  import { ui } from "../../ui";
  import NoteRow from "./note.look.svelte";

  interface Props {
    readonly landing: Landing;
  }

  const { landing }: Props = $props();
  const { lang } = ui();
</script>

{#if landing.kind === "elsewhere"}
  <div class="mb-snug">
    <NoteRow
      role="status"
      pieces={[
        { kind: "words", text: say($lang, "talk_landed"), ink: "faint" },
        { kind: "link", text: landing.addr, href: toFragment({ kind: "talk", address: landing.addr }), face: "mono" },
      ]}
    />
  </div>
{/if}
