<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->
<script lang="ts" module>
  import WaitingCards from "./waiting_cards.svelte";

  export { WaitingCards };
</script>

<script lang="ts">
  import { QUERIES } from "../../core/asking";
  import { ui } from "../../ui";
  import type { ApprovalItem } from "../../wire";

  interface Props {
    readonly items?: readonly ApprovalItem[] | undefined;
  }

  const { items }: Props = $props();

  const answer = ui().conn.asking.ask(QUERIES.approvals);
  const held = $derived(
    items ?? ($answer !== undefined && "approvals" in $answer ? $answer.approvals.items : []),
  );
</script>

<WaitingCards items={held} />
