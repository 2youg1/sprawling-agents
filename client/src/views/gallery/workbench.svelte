<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->
<script lang="ts" module>
  // The world layer as docs/frontend-method.md §7H and client/Spec.lean §7K draw it, in a made-up city
  // of three buildings: the panorama workbench on a building room with a
  // commit picked, the same room in the blend tier, the workbench with
  // the right pane open, and the two readings of the third pane that
  // follow the workspace - the city beside the Mayor's room and the
  // files beside a building's - and, closer, what the wire's run-level
  // fields draw: the room chip open on who is listening, the session
  // pane at the width a panorama gives it, and a picked commit's B3.
  // The panorama is drawn at 1440 and at 1920, and the instrument sheet
  // alone at a phone's 390, because the sheet's two speed cells must
  // hold their figures uncut at each of the three.
  //
  // The room's session ran three turns that read, ran and edited, and
  // checkpointed twice; its building's history holds a branch that
  // another room's session merged back, so the swimlane graph has a
  // lane leaving and rejoining. The session's figures are chosen to be
  // told apart: 1.2 million tokens read, a fifth of them from the cache,
  // a median time to first content of 412 ms. The run works in its own
  // worktree under the tested-and-ordinary policy, its finished command
  // exited 0, and each commit carries the B3 of its checkpoint line.
  import { pickCommit } from "../world/chosen.svelte";
  import { PICKED } from "./benched";

  // The commit the first case shows picked: the session's own last
  // checkpoint, opened under its row.
  if (PICKED !== undefined) pickCommit(PICKED);
</script>

<script lang="ts">
  import { say } from "../../core/lang";
  import { roomOf } from "../../core/route";
  import type { Tier } from "../../core/prefs";
  import { ui } from "../../ui";
  import Edge from "../edge.svelte";
  import Listening from "../talk/listening.svelte";
  import PillView from "../talk/pill.svelte";
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
  const { lang } = u;
  const records = recordsAt(u.now());
  // The room chip as the settings row draws it, held open.
  const chip = $derived({
    label: say($lang, "talk_column_workspace"),
    placeholder: say($lang, "talk_column_workspace"),
    choices: ["hall/mayor", "lab/room1", "lab/room2", "shop/back"].map((room) => ({ value: room, label: room })),
    value: ROOM,
    pick: () => undefined,
  });

  interface Shown {
    readonly label: string;
    readonly tier: Tier;
    readonly panel: boolean;
    readonly width: number;
  }

  const SHOWN: readonly Shown[] = [
    { label: "workbench · panorama, a building room with a commit picked", tier: "panorama", panel: false, width: 1440 },
    { label: "workbench · panorama at 1920", tier: "panorama", panel: false, width: 1920 },
    { label: "workbench · blend, beside the conversation", tier: "blend", panel: false, width: 1440 },
    { label: "workbench · panorama, the right pane open", tier: "panorama", panel: true, width: 1440 },
  ];
</script>

{#each SHOWN as shown (shown.label)}
  <Case label={shown.label} width={shown.width}>
    <Stand link={{ kind: "live", city: "sprawling" }} unread={[]} waiting={[]} answers={answering} {records}>
      <!-- A container named as the shell's body is, so the specimen's
      silver cut is measured on its own width. -->
      <div class="@container/shell">
        <div class="frame relative h-[860px] translate-x-0 overflow-hidden bg-page">
          <Workspace address={ROOM} tier={shown.tier} seat="specimen" panel={shown.panel} />
          <Edge tier={shown.tier} onTier={() => undefined} onPeek={() => undefined} mailboxAsked={0} />
        </div>
      </div>
    </Stand>
  </Case>
{/each}
<Case label="workbench · the city beside the Mayor's room">
  <Stand link={{ kind: "live", city: "sprawling" }} unread={[]} waiting={[]} answers={answering} {records}>
    <div class="flex h-[360px] w-[440px] flex-col">
      <City />
    </div>
  </Stand>
</Case>
<Case label="workbench · a building's files beside its room">
  <Stand link={{ kind: "live", city: "sprawling" }} unread={[]} waiting={[]} answers={answering} {records}>
    <div class="flex h-[360px] w-[440px] flex-col">
      <Files here={ROOM} />
    </div>
  </Stand>
</Case>
{#snippet bare()}{/snippet}
{#snippet listening()}<Listening room={ROOM} />{/snippet}
<Case label="workbench · the room chip open on who is listening">
  <Stand link={{ kind: "live", city: "sprawling" }} unread={[]} waiting={[]} answers={answering} {records}>
    <div class="flex h-[540px] items-end pb-base">
      <PillView spec={chip} starts="open" told={listening} />
    </div>
  </Stand>
</Case>
<Case label="workbench · the chosen session's pane at its panorama width" width={640}>
  <Stand link={{ kind: "live", city: "sprawling" }} unread={[]} waiting={[]} answers={answering} {records}>
    <div class="flex h-[640px] flex-col">
      <Session here={ROOM} title={roomOf(ROOM)} head={bare} />
    </div>
  </Stand>
</Case>
<Case label="workbench · the session's instrument sheet at a phone's width" width={390}>
  <Stand link={{ kind: "live", city: "sprawling" }} unread={[]} waiting={[]} answers={answering} {records}>
    <Sheet here={ROOM} run={run(6)} rounds={{ run: run(6), turns: TURNS, opened_at: oid("a"), opening: OPENING, worktree: "lab-room1" }} />
  </Stand>
</Case>
<Case label="workbench · a picked commit's identity in the commits pane">
  <Stand link={{ kind: "live", city: "sprawling" }} unread={[]} waiting={[]} answers={answering} {records}>
    <div class="flex h-[520px] w-[440px] flex-col">
      <Commits here={ROOM} />
    </div>
  </Stand>
</Case>
