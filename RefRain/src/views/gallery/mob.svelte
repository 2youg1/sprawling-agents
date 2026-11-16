<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->
<script lang="ts">
  
  
  import type { Tier } from "../../core/prefs";
  import { ui } from "../../ui";
  import type { Address as AddressType } from "../../wire";
  import Edge from "../edge.svelte";
  import Workspace from "../workspace.svelte";
  import Case from "./case.svelte";
  import { recordsAt } from "./resulted.svelte";
  import { EMPTY_ROOM, ROOM, answering } from "./shelled";
  import Stand from "./stand.svelte";

  // eslint-disable-next-line @typescript-eslint/no-unsafe-assignment, @typescript-eslint/no-unsafe-call -- typescript-eslint resolves no named export of a `.svelte` module; `svelte-check` resolves it and is the type gate
  const records = recordsAt(ui().now());

  interface Shown {
    readonly label: string;
    readonly room: AddressType;
    readonly tier: Tier;
    readonly panel: boolean;
    readonly width: number;
    readonly height: number;
  }

  const SHOWN: readonly Shown[] = [
    {
      label: "mob · a working room on a phone, 390 by 844",
      room: ROOM,
      tier: "zen",
      panel: false,
      width: 390,
      height: 844,
    },
    {
      label: "mob · an empty room on a phone",
      room: EMPTY_ROOM,
      tier: "zen",
      panel: false,
      width: 390,
      height: 844,
    },
    {
      label: "mob · the world's sheet, the conversation its band",
      room: ROOM,
      tier: "panorama",
      panel: false,
      width: 390,
      height: 844,
    },
    {
      label: "mob · the right side's sheet",
      room: ROOM,
      tier: "zen",
      panel: true,
      width: 390,
      height: 844,
    },
    {
      label: "mob · the soft keyboard open, 500 pixels left",
      room: ROOM,
      tier: "zen",
      panel: false,
      width: 390,
      height: 500,
    },
    {
      label: "mob · a 1440 by 900 window at 200 % zoom",
      room: ROOM,
      tier: "zen",
      panel: false,
      width: 720,
      height: 450,
    },
  ];
</script>

{#each SHOWN as shown (shown.label)}
  <Case label={shown.label} width={shown.width}>
    <Stand
      link={{ kind: "live", city: "sprawling" }}
      unread={[]}
      waiting={[]}
      answers={answering(shown.panel)}
      {records}
    >
      
      <div class="@container/shell">
        <div
          class="frame relative translate-x-0 overflow-hidden bg-page"
          style:height="{String(shown.height)}px"
        >
          <Workspace address={shown.room} tier={shown.tier} seat="specimen" panel={shown.panel} />
          <Edge
            tier={shown.tier}
            onTier={() => undefined}
            onPeek={() => undefined}
            mailboxAsked={0}
          />
        </div>
      </div>
    </Stand>
  </Case>
{/each}
