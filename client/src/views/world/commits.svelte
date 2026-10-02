<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  // The commits pane's graph (client-SPEC 7K, refrain roadmap S7.9): the
  // commits of the place the conversation is in - the whole city from
  // the Mayor's room, the building from a room inside one - newest
  // first, as a swimlane graph drawn from each commit's parents. A row
  // is the graph's node, the short oid, the message and the room that
  // made it; the time is the timeline's, so it is not said here.
  //
  // The chosen session's commits and its lane take the accent (7B); a
  // node takes the phase of the run that made it (`runs/phase.ts`), so a
  // commit whose run is still going reads as going. Picking a row picks
  // its session - the conversation moves to that room when it is another
  // - and opens the commit under its row: its full oid, the B3 of the
  // checkpoint that announced it, its parents and the files it changed,
  // each opening into its patch. Both hashes are written whole on one
  // line and cut by the pane's edge, so a copy takes the whole value.
  import { readAnswer } from "../../core/answered";
  import { fill, say } from "../../core/lang";
  import { MAYOR, buildingOf, roomOf, toFragment } from "../../core/route";
  import { ui } from "../../ui";
  import type { Address, CommitAnswer, RunId } from "../../wire";
  import Changes from "../changes.svelte";
  import Unanswered from "../parts/unanswered.svelte";
  import { phaseOf } from "../runs/lineage";
  import { PHASE_FILL } from "../runs/phase";
  import { commitsIn, pickCommit, pickedCommit, sessionRun } from "./chosen.svelte";
  import { graphOf } from "./lanes";
  import type { Line } from "./lanes";

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

  // An oid as git prints it short.
  const SHORT = 7;
  // One lane's width and the row's height, in the drawing's own pixels.
  const LANE = 14;
  const HEIGHT = 44;
  const MID = HEIGHT / 2;
  const width = $derived(Math.max(1, graph.lanes) * LANE);
  const x = (lane: number): number => LANE / 2 + lane * LANE;

  function path(line: Line): string {
    const [x1, x2] = [x(line.from), x(line.to)];
    switch (line.part) {
      case "through":
        return `M${String(x1)} 0V${String(HEIGHT)}`;
      case "in":
        return `M${String(x1)} 0C${String(x1)} ${String(MID / 2)} ${String(x2)} ${String(MID / 2)} ${String(x2)} ${String(MID)}`;
      case "out":
        return `M${String(x1)} ${String(MID)}C${String(x1)} ${String(MID * 1.5)} ${String(x2)} ${String(MID * 1.5)} ${String(x2)} ${String(HEIGHT)}`;
    }
  }

  const ownRun = (row: number): RunId | undefined => commits[row]?.run;

  function pick(commit: CommitAnswer): void {
    pickCommit(commit);
    if (commit.actor !== here) u.go({ kind: "talk", address: commit.actor });
  }
</script>

<div class="min-h-0 flex-1 overflow-y-auto">
  {#if read.kind === "unavailable"}
    <Unanswered query={read.query} asked={asked} />
  {:else if read.kind === "held" && commits.length === 0}
    <p class="py-snug text-note text-text-faint">{say($lang, "world_no_commits")}</p>
  {:else}
    <ol>
      {#each commits as commit, row (commit.oid)}
        {@const placed = graph.rows[row]}
        {@const mine = commit.run === session}
        {@const doing = $belief.runs[commit.run]?.doing}
        <li>
          <button
            type="button"
            class={[
              "-mr-snug grid h-bar w-full grid-cols-[auto_8ch_minmax(0,1fr)] items-center gap-x-base rounded-card pr-snug text-left text-note",
              commit.oid === picked ? "wash-strong" : "hover:wash",
              commit.actor === here ? "text-text" : "text-text-faint",
            ]}
            aria-expanded={commit.oid === picked}
            onclick={() => {
              pick(commit);
            }}
          >
            <span class="relative h-bar" style:width="{width}px" aria-hidden="true">
              <svg class="absolute inset-0 overflow-visible" width={width} height={HEIGHT} viewBox="0 0 {width} {HEIGHT}">
                {#each placed?.lines ?? [] as line, at (at)}
                  <path
                    d={path(line)}
                    fill="none"
                    stroke-width="2"
                    class={ownRun(line.owner) === session ? "stroke-accent" : "stroke-edge-input"}
                  />
                {/each}
              </svg>
              <span
                class={[
                  "absolute top-1/2 size-dot -translate-x-1/2 -translate-y-1/2 rounded-pill",
                  doing === undefined ? PHASE_FILL.done : PHASE_FILL[phaseOf(doing)],
                  mine ? "ring-2 ring-accent" : "",
                ]}
                style:left="{x(placed?.lane ?? 0)}px"
              ></span>
            </span>
            <span class="figure text-text-quiet">{commit.oid.slice(0, SHORT)}</span>
            <span class="truncate">
              {commit.message ?? ""}
              <span class="ml-snug text-text-faint">{roomOf(commit.actor)}</span>
            </span>
          </button>
          {#if commit.oid === picked}
            {@const parent = commit.parents?.[0]}
            <div class="mb-base ml-[calc(var(--spacing-base)+8ch)] border-l border-edge-panel pl-base text-note">
              <dl class="grid grid-cols-[8ch_minmax(0,1fr)] gap-x-base text-text-quiet">
                <dt class="text-text-faint">{say($lang, "world_commit_oid")}</dt>
                <dd class="figure truncate">{commit.oid}</dd>
                {#if commit.b3 !== undefined && commit.b3 !== null}
                  <dt class="text-text-faint">{say($lang, "world_commit_b3")}</dt>
                  <dd class="figure truncate">{commit.b3}</dd>
                {/if}
                <dt class="text-text-faint">{say($lang, "world_parents")}</dt>
                <dd class="figure truncate">
                  {(commit.parents ?? []).map((oid) => oid.slice(0, SHORT)).join(" · ") || "—"}
                </dd>
              </dl>
              {#if parent !== undefined}
                <div class="mt-snug">
                  <Changes base={parent} head={commit.oid} talk={here} />
                </div>
              {/if}
            </div>
          {/if}
        </li>
      {/each}
    </ol>
    {#if read.kind === "held" && read.value.more && here !== MAYOR}
      <a
        class="block py-snug text-note text-accent hover:text-accent-hover"
        href={toFragment({ kind: "building", address: buildingOf(here) })}
      >
        {fill(say($lang, "world_commits_more"), { building: buildingOf(here) })}
      </a>
    {/if}
  {/if}
</div>
