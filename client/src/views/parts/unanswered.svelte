<script lang="ts" module>
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What stands where a view's answer would be when the city said it
// could not look (`Answer::Unavailable`). It names the question as the
// city spelled it back and offers the one recovery a page has: asking
// again, which a city that has since caught up answers.

import type { Query } from "../../wire";

export interface UnansweredProps {
  // The question as the city spelled it back.
  readonly query: string;
  // Why the city could not look, when it says (`Answer::Unavailable`,
  // wire D47); `null` where it only names the question.
  readonly reason?: string | null | undefined;
  // The question this view asked, sent again by the recovery.
  readonly asked: Query;
}
</script>

<script lang="ts">
  import { fill, say } from "../../core/lang";
  import { ui } from "../../ui";
  import Button from "./button.svelte";

  const { query, asked, reason }: UnansweredProps = $props();

  const u = ui();
  const lang = u.lang;
</script>

<div class="flex flex-col items-start gap-snug text-note">
  <p class="text-text-faint">{fill(say($lang, "answer_unavailable"), { query })}</p>
  {#if reason !== undefined && reason !== null}
    <p class="font-mono text-text-faint">{reason}</p>
  {/if}
  <Button
    label={say($lang, "link_retry")}
    tone="quiet"
    onPress={() => {
      u.conn.asking.refresh(asked);
    }}
  />
</div>
