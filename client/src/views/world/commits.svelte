<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  // The commits pane's graph (client/Spec.lean §7K, refrain roadmap S7.9): the
  // commits of the place the conversation is in - the whole city from
  // the Mayor's room, the building from a room inside one - newest
  // first, as a swimlane graph drawn from each commit's parents. A row
  // is the graph's node, the short oid, the message and the room that
  // made it; the time is the timeline's, so it is not said here.
  //
  // The chosen session's commits and its lane take the accent (docs/frontend-method.md §7B); a
  // node takes the phase of the run that made it (`runs/phase.ts`), so a
  // commit whose run is still going reads as going. Picking a row picks
  // its session - the conversation moves to that room when it is another
  // - opens what it changed against its first parent on the right side
  // (refrain §3-9, client/Spec.lean §4-63), and states the commit under its
  // row: its full oid, the B3 of the checkpoint that announced it and its
  // parents. Both hashes are written whole on one line and cut by the
  // pane's edge, so a copy takes the whole value.
  //
  // This file is the seat: it asks, picks and opens; `./commits.ts`
  // reads each row and `./commits.look.svelte` draws the graph and the
  // rows.
  import { readAnswer } from "../../core/answered";
  import { fill, say } from "../../core/lang";
  import { MAYOR, buildingOf, roomOf, toFragment } from "../../core/route";
  import { ui } from "../../ui";
  import type { Address, CommitAnswer } from "../../wire";
  import { openChanges } from "../inspect/open.svelte";
  import Unanswered from "../parts/unanswered.svelte";
  import { phaseOf } from "../runs/lineage";
  import { commitsIn, pickCommit, pickedCommit, sessionRun } from "./chosen.svelte";
  import { HEIGHT, rowOf, widthOf } from "./commits";
  import type { CommitsLook } from "./commits";
  import Look from "./commits.look.svelte";
  import { graphOf } from "./lanes";

  interface Props {
    readonly here: Address;
  }

  const { here }: Props = $props();

  const u = ui();
  const { lang } = u;
  const belief = u.conn.belief;

  const asked = $derived(commitsIn(here));
  const answer = $derived(u.conn.asking.ask(asked));
  const read = $derived(readAnswer($answer, (held) => ("commits" in held ? held.commits : undefined)));
  const commits = $derived(read.kind === "held" ? read.value.commits : []);
  const graph = $derived(graphOf(commits));

  const session = $derived(sessionRun($belief, here));
  const picked = $derived(pickedCommit(here));

  function pick(commit: CommitAnswer): void {
    pickCommit(commit);
    const parent = commit.parents?.[0];
    if (parent !== undefined) openChanges({ base: parent, head: commit.oid });
    if (commit.actor !== here) u.go({ kind: "talk", address: commit.actor });
  }

  const look: CommitsLook = $derived({
    width: widthOf(graph),
    height: HEIGHT,
    rows: commits.map((commit, row) => {
      const doing = $belief.runs[commit.run]?.doing;
      return rowOf({
        commit,
        graph,
        row,
        picked: commit.oid === picked,
        here: commit.actor === here,
        phase: doing === undefined ? "done" : phaseOf(doing),
        mine: (at) => commits[at]?.run === session,
        room: roomOf(commit.actor),
        pick,
      });
    }),
    labels: { oid: say($lang, "world_commit_oid"), b3: say($lang, "world_commit_b3"), parents: say($lang, "world_parents") },
    more:
      read.kind === "held" && read.value.more && here !== MAYOR
        ? {
            href: toFragment({ kind: "building", address: buildingOf(here) }),
            word: fill(say($lang, "world_commits_more"), { building: buildingOf(here) }),
          }
        : undefined,
  });
</script>

<div class="min-h-0 flex-1 overflow-y-auto">
  {#if read.kind === "unavailable"}
    <Unanswered query={read.query} asked={asked} />
  {:else if read.kind === "held" && commits.length === 0}
    <p class="py-snug text-note text-text-faint">{say($lang, "world_no_commits")}</p>
  {:else}
    <Look {...look} />
  {/if}
</div>
