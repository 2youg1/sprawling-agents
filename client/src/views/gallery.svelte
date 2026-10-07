<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  // Every state worth looking at, on fixtures, with no city behind it.
  //
  // It is a route rather than a build flag for two reasons: the gate
  // that measures it opens the same bundle a person runs, and a person
  // deciding between two ways a thing could look can open it on their
  // own machine without standing up a provider first.
  //
  // **This file is the index and nothing else.** Each section under
  // `gallery/` owns one subject - the room a person works in, what a
  // run produced, the registry, the shared controls - together with
  // the fixture values that subject needs, so a fixture is read beside
  // the component it is about rather than four hundred lines above it.
  // The order below is the order a reader meets them in, and
  // `gallery/case.svelte` holds the one wrapper they all draw into.
  //
  // **The hints lead on purpose.** A hint opens above its control and
  // flips below when the window has no room above, so the one fixture
  // that can exercise the flip is the one nearest the top of the
  // window; everything after it is ordered for a person reading down
  // the page.
</script>

<script lang="ts">
  import { onMount } from "svelte";

  import { say } from "../core/lang";
  import { setUi, ui } from "../ui";
  import Accounted from "./gallery/accounted.svelte";
  import Searched from "./gallery/searched.svelte";
  import Attaching from "./gallery/attaching.svelte";
  import Anchored from "./gallery/anchored.svelte";
  import Conversation from "./gallery/conversation.svelte";
  import Doc from "./gallery/doc.svelte";
  import Filed from "./gallery/filed.svelte";
  import FirstRun from "./gallery/first_run.svelte";
  import G2 from "./gallery/g2.svelte";
  import Gd from "./gallery/gd.svelte";
  import G6 from "./gallery/g6.svelte";
  import Fmt from "./gallery/fmt.svelte";
  import G1 from "./gallery/g1.svelte";
  import G3 from "./gallery/g3.svelte";
  import Folded from "./gallery/folded.svelte";
  import Followed from "./gallery/followed.svelte";
  import G5 from "./gallery/g5.svelte";
  import Hints from "./gallery/hints.svelte";
  import Inspected from "./gallery/ins.svelte";
  import Keepers from "./gallery/kept.svelte";
  import RefusedLine from "./gallery/refused_line.svelte";
  import Prop from "./gallery/prop.svelte";
  import PrivacyGallery from "./gallery/privacy.svelte";
  import Rfr from "./gallery/rfr.svelte";
  import Ring from "./gallery/ring.svelte";
  import Md from "./gallery/md.svelte";
  import Monitor from "./gallery/monitor.svelte";
  import PartsBadge from "./gallery/parts/badge.svelte";
  import PartsCopy from "./gallery/parts/copy.svelte";
  import PartButton from "./gallery/parts/button.svelte";
  import PartCombobox from "./gallery/parts/combobox.svelte";
  import PartDialog from "./gallery/parts/dialog.svelte";
  import PartField from "./gallery/parts/field.svelte";
  import PartNotice from "./gallery/parts/notice.svelte";
  import PartPath from "./gallery/parts/path.svelte";
  import PartPopover from "./gallery/parts/popover.svelte";
  import PartSegmented from "./gallery/parts/segmented.svelte";
  import PartTable from "./gallery/parts/table.svelte";
  import PartTip from "./gallery/parts/tip.svelte";
  import Mailbox from "./gallery/mailbox.svelte";
  import Mob from "./gallery/mob.svelte";
  import Pga from "./gallery/pga.svelte";
  import Pgb from "./gallery/pgb.svelte";
  import R3 from "./gallery/r3.svelte";
  import Produced from "./gallery/produced.svelte";
  import Runs from "./gallery/runs.svelte";
  import Resulted from "./gallery/resulted.svelte";
  import Streamed from "./gallery/streamed.svelte";
  import Screens from "./gallery/screens.svelte";
  import Settings from "./gallery/settings.svelte";
  import SettingsPanel from "./gallery/set.svelte";
  import Shell from "./gallery/shell.svelte";
  import Shelved from "./gallery/shelved.svelte";
  import TabsCases from "./gallery/parts/tabs.svelte";
  import Talking from "./gallery/talking.svelte";
  import TalkRow from "./gallery/talk_row.svelte";
  import Thr from "./gallery/thr.svelte";
  import AgentMessages from "./gallery/agent_messages.svelte";
  import AgentLetters from "./gallery/agent_letters.svelte";
  import AgentChildren from "./gallery/agent_children.svelte";
  import AgentWaits from "./gallery/agent_waits.svelte";
  import Tables from "./gallery/tables.svelte";
  import PartRow from "./gallery/parts/row.svelte";
  import Monitored from "./gallery/monitored.svelte";
  import Timed from "./gallery/timed.svelte";
  import Tok from "./gallery/tok.svelte";
  import Read from "./gallery/read.svelte";
  import TipPlaced from "./gallery/parts/tip_placed.svelte";
  import DialogDrawn from "./gallery/parts/dialog_drawn.svelte";
  import KbdMarks from "./gallery/parts/kbd.svelte";
  import Deciding from "./gallery/parts/decide.svelte";
  import Used from "./gallery/used.svelte";
  import Workbench from "./gallery/workbench.svelte";
  import Ss from "./gallery/ss.svelte";
  import World from "./gallery/world.svelte";

  const { lang } = ui();

  // The city as the shell opened it, captured before any stand below
  // installs one of its own. `Stand` hands `ui.ts`'s one door to a
  // fixture subtree while that subtree initialises; this closes the
  // door once the route has mounted, so a component arriving later -
  // the next page a person walks to, or a panel opened under a fixture
  // - meets the real city and not the last stand made.
  const real = ui();
  onMount(() => {
    setUi({
      conn: real.conn,
      prefs: real.prefs,
      bar: real.bar,
      origin: real.origin,
      pairing: real.pairing,
      now: real.now,
    });
  });
</script>

<div class="w-full min-w-0 px-pane py-pane">
  <h1 class="mb-wide text-heading text-text">{say($lang, "gallery_title")}</h1>
  <Hints />
  <FirstRun />
  <Pgb />
  <Shell />
  <Mob />
  <Workbench />
  <Ss />
  <World />
  <Mailbox />
  <Conversation />
  <Md />
  <RefusedLine />
  <Talking />
  <Ring />
  <Thr />
  <AgentMessages />
  <AgentLetters />
  <AgentChildren />
  <AgentWaits />
  <G1 />
  <Folded />
  <Followed />
  <Anchored />
  <Produced />
  <Inspected />
  <G5 />
  <Filed />
  <Pga />
  <Screens />
  <Accounted />
  <Searched />
  <Attaching />
  <Runs />
  <Shelved />
  <Used />
  <Keepers />
  <Settings />
  <SettingsPanel />
  <PrivacyGallery />
  <G2 />
  <TalkRow />
  <R3 />
  <Doc />
  <Tables />
  <PartRow />
  <Monitor />
  <PartsBadge />
  <PartsCopy />
  <TabsCases />
  <PartButton />
  <PartField />
  <PartPath />
  <PartCombobox />
  <PartTable />
  <PartDialog />
  <PartPopover />
  <PartTip />
  <PartNotice />
  <PartSegmented />
  <Monitored />
  <Timed />
  <Resulted />
  <Streamed />
  <Tok />
  <Read />
  <TipPlaced />
  <DialogDrawn />
  <KbdMarks />
  <Deciding />
  <Rfr />
  <Fmt />
  <G3 />
  <Prop />
  <Gd />
  <G6 />
</div>
