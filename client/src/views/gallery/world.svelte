<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->
<script lang="ts">
  // The world layer's controls one at a time (client/Spec.lean §7K), each
  // drawn by its own seat so the look it hands its wiring to is measured:
  // the timeline where its rows take two lines, a pane label with its menu
  // open and as words in the blend tier, a divider on the shell's frame,
  // and the head of the world's sheet on a phone. The made-up city is the
  // workbench fixture's (`benched.ts`).
  import { say } from "../../core/lang";
  import { roomOf } from "../../core/route";
  import { ui } from "../../ui";
  import Divider from "../world/divider.svelte";
  import PaneMenu from "../world/pane_menu.svelte";
  import SheetHead from "../world/sheet_head.svelte";
  import Timeline from "../world/timeline.svelte";
  import { COMMITS, PICKED, ROOM, TURNS, answering, recordsAt, run } from "./benched";
  import Case from "./case.svelte";
  import { pressing } from "./pressing";
  import Stand from "./stand.svelte";

  const u = ui();
  const { lang } = u;
  const records = recordsAt(u.now());
</script>

<Case label="workbench · the timeline at a phone's width, a checkpoint picked" width={390}>
  <Stand link={{ kind: "live", city: "sprawling" }} unread={[]} waiting={[]} answers={answering} {records}>
    <div class="flex h-[420px] flex-col">
      <Timeline run={run(6)} turns={TURNS} commits={COMMITS} picked={PICKED?.oid ?? null} />
    </div>
  </Stand>
</Case>
<Case label="workbench · a pane's label, its menu open on the middle pane" width={640}>
  <div class="flex h-[160px] flex-col px-base" {@attach pressing('button[aria-haspopup="menu"]')}>
    <PaneMenu pane="session" label={say($lang, "world_session")} arranged="menu" />
  </div>
</Case>
<Case label="workbench · a pane's label in the blend tier, words only" width={640}>
  <div class="flex flex-col px-base">
    <PaneMenu pane="sessions" label={say($lang, "world_sessions")} arranged="words" />
  </div>
</Case>
<Case label="workbench · the divider between the first two panes" width={1440}>
  <!-- The shell's frame, so the divider snaps to the lines it reads. -->
  <div class="frame grid h-[160px] grid-rows-[minmax(0,1fr)_auto]">
    <div class="row-[1/3] rounded-panel border border-edge-panel" style:grid-column="1 / 4"></div>
    <Divider divider={0} label={say($lang, "world_sessions")} controls="gallery-sessions" column={4} />
    <div class="row-[1/3] rounded-panel border border-edge-panel" style:grid-column="4 / 10"></div>
  </div>
</Case>
<Case label="workbench · the world sheet's head on a phone" width={390}>
  <SheetHead
    tabs={[
      { pane: "sessions", label: say($lang, "world_sessions") },
      { pane: "session", label: say($lang, "world_session") },
      { pane: "commits", label: roomOf(ROOM) },
    ]}
    shown="commits"
    prefix="gallery-sheet"
    onShow={() => undefined}
  />
</Case>
