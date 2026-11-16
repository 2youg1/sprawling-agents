<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  import { ui } from "../../ui";
  import type { Address } from "../../wire";
  import Talk from "../talk.svelte";
  import { SPECIMEN } from "../talk/seat";
  import Case from "./case.svelte";
  import {
    answering,
    FINISHED,
    LEFT,
    LETTERS,
    OLDER,
    QUEUED,
    recordsAt,
    REPLIED,
    SENDS,
    STOPPED,
    TIMED_OUT,
    WAITING,
  } from "./agent_letters";
  import Stand from "./stand.svelte";

  const u = ui();
  const now = u.now();
  const records = recordsAt(now);
  const answers = answering(now);

  const SHOWN: readonly (readonly [string, Address])[] = [
    ["agent messages · the User's steer beside a resident's letter, and a pulled signal", LETTERS],
    ["agent messages · a reply wait in progress", WAITING],
    ["agent messages · a reply wait ended by the reply", REPLIED],
    ["agent messages · a reply wait ended by the timeout", TIMED_OUT],
    ["agent messages · a reply wait ended because the room left", LEFT],
    ["agent messages · a handback, finished", FINISHED],
    ["agent messages · a handback, stopped", STOPPED],
    ["agent messages · two queued signals with their first lines", QUEUED],
    ["agent messages · a letter from an older Ledger, without its kind or session", OLDER],
    ["agent messages · send lines: delivered, queued, knocked a new run, and not recorded", SENDS],
  ];
</script>

{#each SHOWN as [label, room] (room)}
  <Case {label} width={760}>
    <Stand link={{ kind: "live", city: "sprawling" }} unread={[]} waiting={[]} {answers} {records}>
      <div class="h-[440px] px-wide">
        <Talk address={room} band={false} seat={SPECIMEN} />
      </div>
    </Stand>
  </Case>
{/each}
