<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  import { fill, say } from "../../core/lang";
  import { ui } from "../../ui";
  import type { UnansweredProps } from "./unanswered";
  import Look from "./unanswered.look.svelte";

  const { query, asked, reason }: UnansweredProps = $props();

  const u = ui();
  const lang = u.lang;
</script>

<Look
  said={fill(say($lang, "answer_unavailable"), { query })}
  reason={reason ?? undefined}
  retry={{
    label: say($lang, "link_retry"),
    press: () => {
      u.conn.asking.refresh(asked);
    },
  }}
/>
