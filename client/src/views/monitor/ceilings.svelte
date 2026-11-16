<script lang="ts" module>
  // This Source Code Form is subject to the terms of the Mozilla Public
  // License, v. 2.0. If a copy of the MPL was not distributed with this
  // file, You can obtain one at https://mozilla.org/MPL/2.0/.
  // Copyright (c) 2026 2youg1 and the sprawling contributors
</script>

<script lang="ts">
  import { fill, say, type Key } from "../../core/lang";
  import { count } from "../../core/time";
  import { ui } from "../../ui";
  import type { CeilingNote, CeilingWhy } from "./trace";

  interface Props {
    readonly notes: readonly CeilingNote[];
  }

  const { notes }: Props = $props();
  const { lang } = ui();

  const WHY: Record<CeilingWhy, Key> = {
    platform: "mon_ceiling_why_platform",
    not_delegated: "mon_ceiling_why_not_delegated",
    refused: "mon_ceiling_why_refused",
    unjoined: "mon_ceiling_why_unjoined",
  };

  function said(note: CeilingNote): string {
    const limit = count(note.ceiling.limit);
    const line = (() => {
      switch (note.ceiling.state) {
        case "hit":
          return fill(say($lang, "mon_ceiling_hit"), { limit });
        case "unapplied":
          return fill(say($lang, "mon_ceiling_unapplied"), {
            limit,
            why: say($lang, WHY[note.ceiling.why]),
          });
        case "unread":
          return fill(say($lang, "mon_ceiling_unread"), { limit });
      }
    })();
    return note.handle === null
      ? line
      : fill(say($lang, "mon_ceiling_of"), { handle: note.handle, line });
  }
</script>

{#each notes as note, n (n)}
  <p
    data-ceiling={note.ceiling.state}
    class="ps-[2ch] -indent-[2ch] whitespace-pre-wrap wrap-break-word text-alert"
  >
    {said(note)}
  </p>
{/each}
