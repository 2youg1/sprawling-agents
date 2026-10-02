<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->
<script lang="ts" module>
  // The shell, whole, in the states client-SPEC 7H and 7I name: an empty
  // room, a working room in each of the three tiers, and the right pane
  // open. Every case mounts the conversation page and the edge keys the
  // shell mounts, on the shell's own grid at a window of 1440 by 860, in
  // a made-up city (`stand.svelte`, `shelled.ts`): the runs of
  // `resulted.svelte`'s six rooms, a session whose last turn used a fifth
  // of a 204,800-token window, the two reminders at 30% and 65%, and a
  // page of commits.
  import { recordsAt } from "./resulted.svelte";
  import { EMPTY_ROOM, ROOM, answering } from "./shelled";
</script>

<script lang="ts">
  import type { Tier } from "../../core/prefs";
  import { ui } from "../../ui";
  import type { Address as AddressType } from "../../wire";
  import Edge from "../edge.svelte";
  import Workspace from "../workspace.svelte";
  import Case from "./case.svelte";
  import Stand from "./stand.svelte";

  // eslint-disable-next-line @typescript-eslint/no-unsafe-assignment, @typescript-eslint/no-unsafe-call -- typescript-eslint resolves no named export of a `.svelte` module; `svelte-check` resolves it and is the type gate
  const records = recordsAt(ui().now());

  interface Shown {
    readonly label: string;
    readonly room: AddressType;
    readonly tier: Tier;
    readonly panel: boolean;
  }

  const SHOWN: readonly Shown[] = [
    { label: "shell · an empty room, in zen", room: EMPTY_ROOM, tier: "zen", panel: false },
    { label: "shell · a working room, in zen", room: ROOM, tier: "zen", panel: false },
    { label: "shell · a working room, in blend", room: ROOM, tier: "blend", panel: false },
    { label: "shell · a working room, in panorama", room: ROOM, tier: "panorama", panel: false },
    { label: "shell · the right pane open, in zen", room: ROOM, tier: "zen", panel: true },
  ];
</script>

{#each SHOWN as shown (shown.label)}
  <Case label={shown.label} width={1440}>
    <Stand link={{ kind: "live", city: "sprawling" }} unread={[]} waiting={[]} answers={answering(shown.panel)} {records}>
      <!-- A transform makes this box the containing block of everything
      fixed inside it, so the drawer and the toasts stay in the specimen. -->
      <div class="frame relative h-[860px] translate-x-0 overflow-hidden bg-page">
        <Workspace address={shown.room} tier={shown.tier} seat="specimen" panel={shown.panel} />
        <Edge tier={shown.tier} onTier={() => undefined} onPeek={() => undefined} mailboxAsked={0} />
      </div>
    </Stand>
  </Case>
{/each}
