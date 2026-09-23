<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->
<script lang="ts" module>
  // What waits for the person, as cards in the conversation rather than
  // a page of their own: a question that stops a run belongs where the
  // person is already looking. This file asks for the queue; the cards
  // are `waiting_cards.svelte`, which the gallery seats on its own.
  import WaitingCards from "./waiting_cards.svelte";

  export { WaitingCards };
</script>

<script lang="ts">
  import { QUERIES } from "../../core/asking";
  import { ui } from "../../ui";
  import type { ApprovalItem } from "../../wire";

  interface Props {
    // The queue, already read by the caller. Absent means this file
    // asks for it itself, which is the talk page's one call site: two
    // handlings of the same answer would be two readings of one fact.
    readonly items?: readonly ApprovalItem[] | undefined;
  }

  const { items }: Props = $props();

  const answer = ui().conn.asking.ask(QUERIES.approvals);
  const held = $derived(
    items ??
      ($answer !== undefined && "approvals" in $answer ? $answer.approvals.items : []),
  );
</script>

<WaitingCards items={held} />
