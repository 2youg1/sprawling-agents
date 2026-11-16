<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->
<script lang="ts" module>
  
  
  import { pickCommit } from "../world/chosen.svelte";
  import { PICKED } from "./benched";

  
  if (PICKED !== undefined) pickCommit(PICKED);
</script>

<script lang="ts">
  import type { Tier } from "../../core/prefs";
  import { ui } from "../../ui";
  import Edge from "../edge.svelte";
  import Listening from "../talk/listening.svelte";
  import Workspace from "../workspace.svelte";
  import City from "../world/city.svelte";
  import Commits from "../world/commits.svelte";
  import Files from "../world/files.svelte";
  import Session from "../world/session.svelte";
  import Sheet from "../world/sheet.svelte";
  import { OPENING, ROOM, TURNS, answering, oid, recordsAt, run } from "./benched";
  import Case from "./case.svelte";
  import Stand from "./stand.svelte";

  const u = ui();
  const records = recordsAt(u.now());

  interface Shown {
    readonly label: string;
    readonly tier: Tier;
    readonly panel: boolean;
    readonly width: number;
  }

  const SHOWN: readonly Shown[] = [
    {
      label: "workbench · panorama, a building room with a commit picked",
      tier: "panorama",
      panel: false,
      width: 1440,
    },
    { label: "workbench · panorama at 1920", tier: "panorama", panel: false, width: 1920 },
    {
      label: "workbench · blend, beside the conversation",
      tier: "blend",
      panel: false,
      width: 1440,
    },
    {
      label: "workbench · panorama, the right pane open",
      tier: "panorama",
      panel: true,
      width: 1440,
    },
  ];
</script>

{#each SHOWN as shown (shown.label)}
  <Case label={shown.label} width={shown.width}>
    <Stand
      link={{ kind: "live", city: "sprawling" }}
      unread={[]}
      waiting={[]}
      answers={answering}
      {records}
    >
      
      <div class="@container/shell">
        <div class="frame relative h-[860px] translate-x-0 overflow-hidden bg-page">
          <Workspace address={ROOM} tier={shown.tier} seat="specimen" panel={shown.panel} />
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
<Case label="workbench · the city beside the Mayor's room">
  <Stand
    link={{ kind: "live", city: "sprawling" }}
    unread={[]}
    waiting={[]}
    answers={answering}
    {records}
  >
    <div class="flex h-[360px] w-[440px] flex-col">
      <City />
    </div>
  </Stand>
</Case>
<Case label="workbench · a building's files beside its room">
  <Stand
    link={{ kind: "live", city: "sprawling" }}
    unread={[]}
    waiting={[]}
    answers={answering}
    {records}
  >
    <div class="flex h-[360px] w-[440px] flex-col">
      <Files here={ROOM} />
    </div>
  </Stand>
</Case>
{#snippet bare()}{/snippet}
<Case label="listening · who hears a message in this room, as the palette's foot draws it">
  <Stand
    link={{ kind: "live", city: "sprawling" }}
    unread={[]}
    waiting={[]}
    answers={answering}
    {records}
  >
    <Listening room={ROOM} />
  </Stand>
</Case>
<Case label="workbench · the chosen session's pane at its panorama width" width={640}>
  <Stand
    link={{ kind: "live", city: "sprawling" }}
    unread={[]}
    waiting={[]}
    answers={answering}
    {records}
  >
    <div class="flex h-[640px] flex-col">
      <Session here={ROOM} head={bare} />
    </div>
  </Stand>
</Case>
<Case label="workbench · the session's instrument sheet at a phone's width" width={390}>
  <Stand
    link={{ kind: "live", city: "sprawling" }}
    unread={[]}
    waiting={[]}
    answers={answering}
    {records}
  >
    <Sheet
      here={ROOM}
      run={run(6)}
      rounds={{
        run: run(6),
        turns: TURNS,
        opened_at: oid("a"),
        opening: OPENING,
        worktree: "lab-room1",
      }}
    />
  </Stand>
</Case>
<Case label="workbench · a picked commit's identity in the commits pane">
  <Stand
    link={{ kind: "live", city: "sprawling" }}
    unread={[]}
    waiting={[]}
    answers={answering}
    {records}
  >
    <div class="flex h-[520px] w-[440px] flex-col">
      <Commits here={ROOM} />
    </div>
  </Stand>
</Case>
